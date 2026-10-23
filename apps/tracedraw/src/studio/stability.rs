//! The studio's adapter for Stability AI (docs.stability.ai/api-reference,
//! read 2026-10-23): `https://api.stability.ai/v2beta`, `Authorization:
//! Bearer`, multipart bodies up to 10 MiB, `Accept: application/json` so
//! pictures come back as base64 in `image`. Creative upscale and Replace
//! Background and Relight run in the background: the POST answers `{id}`
//! and `GET /v2beta/results/{id}` answers 202 until the picture is ready
//! (looked at every 10 s, as the service asks). Prompts must be in
//! English (the studio translates them, see `app.rs`).

use serde_json::Value;

use super::transport::{self, Transport, BOUNDARY};
use super::{fit_image, Op, OpRequest, Output};
use crate::i18n::tr;

pub const BASE: &str = "https://api.stability.ai";

/// The most pixels most edits take, and the most a few take.
const MAX_PIXELS: u64 = 9_437_184;
const ONE_MP: u64 = 1_048_576;
/// How often a background task is looked at, and for how long.
const POLL_SECS: f64 = 10.0;
const POLL_TRIES: u32 = 60;

fn headers(key: &str) -> Vec<(String, String)> {
    vec![
        ("authorization".into(), format!("Bearer {key}")),
        ("accept".into(), "application/json".into()),
        ("stability-client-id".into(), "tracedraw-arcano".into()),
    ]
}

/// The endpoint and the model field of a generation model.
fn generate_endpoint(model: &str) -> (&'static str, Option<&str>) {
    match model {
        "stable-image-ultra" => ("/v2beta/stable-image/generate/ultra", None),
        "stable-image-core" => ("/v2beta/stable-image/generate/core", None),
        m => ("/v2beta/stable-image/generate/sd3", Some(m)),
    }
}

/// The request for `req` (the first one, for operations that run in the
/// background).
pub fn build(key: &str, req: &OpRequest) -> Result<crate::net::HttpRequest, String> {
    let mut fields: Vec<(&str, String)> = vec![("output_format", "png".into())];
    let mut files: Vec<(&str, &str, &str, Vec<u8>)> = Vec::new();
    let image = |max: u64, max_side: u32| -> Result<Vec<u8>, String> {
        let png = req.image.as_deref().ok_or_else(|| tr("aiimg.need_bitmap"))?;
        fit_image(png, max, max_side, 64).map(|(b, _, _)| b)
    };
    let path = match req.op() {
        Op::Generate => {
            let (path, model) = generate_endpoint(&req.model);
            fields.push(("prompt", req.prompt.clone()));
            fields.push(("aspect_ratio", req.aspect.clone()));
            if let Some(m) = model {
                fields.push(("model", m.to_string()));
            }
            path
        }
        Op::Remix => {
            let (path, model) = generate_endpoint(&req.model);
            if req.model == "stable-image-core" {
                return Err(tr("studio.model_no_remix"));
            }
            fields.push(("prompt", req.prompt.clone()));
            fields.push(("strength", format!("{}", req.num("strength", 0.6))));
            if let Some(m) = model {
                fields.push(("model", m.to_string()));
                fields.push(("mode", "image-to-image".into()));
            }
            files.push(("image", "image.png", "image/png", image(MAX_PIXELS, 0)?));
            path
        }
        Op::Upscale => match req.variant.as_str() {
            "fast" => {
                files.push(("image", "image.png", "image/png", image(ONE_MP, 1536)?));
                "/v2beta/stable-image/upscale/fast"
            }
            "creative" => {
                fields.push(("prompt", prompt_or(req, "high quality, sharp, detailed")));
                fields.push(("creativity", format!("{}", req.num("creativity", 0.3).clamp(0.1, 0.5))));
                files.push(("image", "image.png", "image/png", image(ONE_MP, 0)?));
                "/v2beta/stable-image/upscale/creative"
            }
            _ => {
                fields.push(("prompt", prompt_or(req, "high quality, sharp, detailed")));
                fields.push(("creativity", format!("{}", req.num("creativity", 0.35).clamp(0.2, 0.5))));
                files.push(("image", "image.png", "image/png", image(MAX_PIXELS, 0)?));
                "/v2beta/stable-image/upscale/conservative"
            }
        },
        Op::Expand => {
            let side = |n: &str| req.num(n, 0.0).clamp(0.0, 2000.0).round() as i64;
            fields.push(("left", side("left").to_string()));
            fields.push(("right", side("right").to_string()));
            fields.push(("up", side("top").to_string()));
            fields.push(("down", side("bottom").to_string()));
            if !req.prompt.trim().is_empty() {
                fields.push(("prompt", req.prompt.clone()));
            }
            files.push(("image", "image.png", "image/png", image(MAX_PIXELS, 0)?));
            "/v2beta/stable-image/edit/outpaint"
        }
        Op::RemoveBackground => {
            files.push(("image", "image.png", "image/png", image(4_194_304, 0)?));
            "/v2beta/stable-image/edit/remove-background"
        }
        Op::ReplaceBackground | Op::Relight => {
            let relight = req.op() == Op::Relight;
            fields.push(("background_prompt", req.prompt.clone()));
            if relight {
                fields.push(("keep_original_background", "true".into()));
                fields.push(("light_source_direction", req.text("direction", "above").to_string()));
                fields.push(("light_source_strength", format!("{}", req.num("strength", 0.5).clamp(0.0, 1.0))));
            } else {
                fields.push(("preserve_original_subject", format!("{}", req.num("preserve", 0.6).clamp(0.0, 1.0))));
            }
            files.push(("subject_image", "image.png", "image/png", image(MAX_PIXELS, 0)?));
            "/v2beta/stable-image/edit/replace-background-and-relight"
        }
        Op::Vectorize => return Err(tr("studio.not_offered")),
    };
    let file_refs: Vec<(&str, &str, &str, &[u8])> =
        files.iter().map(|(a, b, c, d)| (*a, *b, *c, d.as_slice())).collect();
    let body = transport::multipart(BOUNDARY, &fields, &file_refs);
    if body.len() > 10 * 1024 * 1024 {
        return Err(tr("studio.too_large"));
    }
    Ok(transport::post_multipart(format!("{BASE}{path}"), headers(key), body))
}

/// The operation runs in the background (the answer is a task).
fn in_background(req: &OpRequest) -> bool {
    matches!(req.op(), Op::ReplaceBackground | Op::Relight)
        || (req.op() == Op::Upscale && req.variant == "creative")
}

fn prompt_or(req: &OpRequest, default: &str) -> String {
    if req.prompt.trim().is_empty() {
        default.to_string()
    } else {
        req.prompt.clone()
    }
}

/// The picture in an answer: `image` in base64; a picture blurred by the
/// service's moderation is refused.
fn picture(v: &Value) -> Result<Vec<u8>, String> {
    if v.get("finish_reason").and_then(|f| f.as_str()) == Some("CONTENT_FILTERED") {
        return Err(tr("studio.filtered"));
    }
    v.get("image")
        .and_then(|i| i.as_str())
        .and_then(transport::unbase64)
        .ok_or_else(|| tr("studio.bad_reply"))
}

pub fn run(t: &dyn Transport, key: &str, req: &OpRequest) -> Result<Output, String> {
    let http = build(key, req)?;
    let v = transport::json_ok(t.call(&http))?;
    if !in_background(req) {
        return Ok(Output::Images(vec![picture(&v)?]));
    }
    let id = v
        .get("id")
        .and_then(|i| i.as_str())
        .ok_or_else(|| tr("studio.bad_reply"))?
        .to_string();
    for _ in 0..POLL_TRIES {
        t.wait(POLL_SECS);
        let r = t.call(&transport::get(format!("{BASE}/v2beta/results/{id}"), headers(key)))?;
        match r.status {
            202 => continue,
            200 => {
                let v: Value =
                    serde_json::from_str(&r.body).map_err(|_| tr("studio.bad_reply"))?;
                return Ok(Output::Images(vec![picture(&v)?]));
            }
            s => {
                let v: Option<Value> = serde_json::from_str(&r.body).ok();
                return Err(transport::service_error(s, v.as_ref(), &r.body));
            }
        }
    }
    Err(tr("studio.timed_out"))
}

/// The account's credits (`GET /v1/user/balance`).
pub fn test_key(t: &dyn Transport, key: &str) -> Result<String, String> {
    let v = transport::json_ok(t.call(&transport::get(format!("{BASE}/v1/user/balance"), headers(key))))?;
    let credits = v.get("credits").and_then(|c| c.as_f64()).unwrap_or(0.0);
    Ok(crate::i18n::trf(
        "studio.credits",
        &[("credits", &crate::ai_image::localize_decimal(format!("{credits:.1}")))],
    ))
}

#[cfg(test)]
mod tests {
    use super::super::transport::{reply, Fake};
    use super::*;
    use serde_json::json;

    fn png_b64() -> String {
        transport::base64(&super::super::test_png(8, 8))
    }

    fn body_text(r: &crate::net::HttpRequest) -> String {
        String::from_utf8_lossy(&r.bytes).to_string()
    }

    #[test]
    fn generation_goes_to_the_models_endpoint_with_its_fields() {
        let req = OpRequest {
            op: Some(Op::Generate),
            provider: "stability".into(),
            model: "sd3.5-large".into(),
            prompt: "a red fox".into(),
            aspect: "16:9".into(),
            ..Default::default()
        };
        let r = build("sk", &req).expect("request");
        assert_eq!(r.url, "https://api.stability.ai/v2beta/stable-image/generate/sd3");
        let b = body_text(&r);
        assert!(b.contains("name=\"model\"\r\n\r\nsd3.5-large") && b.contains("name=\"aspect_ratio\"\r\n\r\n16:9"));
        assert!(r.headers.contains(&("authorization".into(), "Bearer sk".into())));
        let ultra = OpRequest { model: "stable-image-ultra".into(), ..req };
        let r = build("sk", &ultra).expect("request");
        assert!(r.url.ends_with("/generate/ultra") && !body_text(&r).contains("name=\"model\""));
    }

    #[test]
    fn edits_name_their_fields_as_the_service_does() {
        let png = super::super::test_png(3000, 2000);
        let req = OpRequest {
            op: Some(Op::Expand),
            image: Some(png.clone()),
            params: [("left".to_string(), json!(100)), ("bottom".to_string(), json!(50))].into_iter().collect(),
            ..Default::default()
        };
        let b = body_text(&build("k", &req).expect("request"));
        assert!(b.contains("name=\"left\"\r\n\r\n100") && b.contains("name=\"down\"\r\n\r\n50"));
        // Fast upscale takes at most 1 MP: the picture is made smaller.
        let up = OpRequest { op: Some(Op::Upscale), variant: "fast".into(), image: Some(png), ..Default::default() };
        let r = build("k", &up).expect("request");
        assert!(r.url.ends_with("/upscale/fast"));
        let start = r.bytes.windows(4).position(|w| w == b"\x89PNG").expect("png in body");
        let (w, h) = super::super::image_size(&r.bytes[start..]).expect("size");
        assert!(w as u64 * h as u64 <= ONE_MP && w <= 1536);
    }

    #[test]
    fn a_background_task_is_looked_at_until_its_picture_is_ready() {
        fn answer(r: &crate::net::HttpRequest) -> crate::net::HttpResult {
            if r.method == "POST" {
                return reply(200, json!({"id": "abc"}));
            }
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if N.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 2 {
                reply(202, json!({"id": "abc", "status": "in-progress"}))
            } else {
                reply(200, json!({"image": png_b64(), "finish_reason": "SUCCESS"}))
            }
        }
        let fake = Fake::new(answer, |_| Err("no".into()));
        let req = OpRequest {
            op: Some(Op::ReplaceBackground),
            prompt: "a beach".into(),
            image: Some(super::super::test_png(200, 200)),
            ..Default::default()
        };
        let out = run(&fake, "k", &req).expect("picture");
        assert!(matches!(out, Output::Images(ref v) if v.len() == 1));
        let sent = fake.sent();
        assert_eq!(sent.len(), 4);
        assert!(sent[1].url.ends_with("/v2beta/results/abc"));
    }

    #[test]
    fn errors_and_moderation_are_reported() {
        let fake = Fake::new(|_| reply(400, json!({"id": "x", "name": "bad_request", "errors": ["prompt: is required"]})), |_| Err("no".into()));
        let req = OpRequest { op: Some(Op::Generate), model: "stable-image-core".into(), ..Default::default() };
        assert!(run(&fake, "k", &req).expect_err("error").contains("prompt: is required"));
        let fake = Fake::new(|_| reply(200, json!({"image": "", "finish_reason": "CONTENT_FILTERED"})), |_| Err("no".into()));
        assert_eq!(run(&fake, "k", &req).expect_err("filtered"), tr("studio.filtered"));
        let fake = Fake::new(|_| reply(200, json!({"credits": 12.5})), |_| Err("no".into()));
        assert!(test_key(&fake, "k").expect("works").contains("12"));
    }
}
