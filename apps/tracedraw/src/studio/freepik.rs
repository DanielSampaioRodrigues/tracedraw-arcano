//! The studio's adapter for the Magnific API, formerly the Freepik API
//! (docs.magnific.com, read 2026-10-23): `https://api.magnific.com/v1`,
//! `x-magnific-api-key`, JSON bodies with pictures in base64. Nearly
//! everything runs in the background: the POST answers
//! `{data: {task_id, status}}` and `GET <same path>/{task_id}` answers
//! `CREATED`, `IN_PROGRESS`, `COMPLETED` (with `generated`, links valid
//! for 30 minutes, downloaded at once) or `FAILED`. Remove Background
//! takes a link only, so the picture is first uploaded (`uploads`).
//! Besides its own Mystic and the Magnific upscalers, relight and
//! expand, the service offers other companies' models with the same key;
//! the catalog lists the ones offered here.

use serde_json::{json, Value};

use super::transport::{self, Transport};
use super::{fit_image, Op, OpRequest, Output};
use crate::i18n::tr;

pub const BASE: &str = "https://api.magnific.com/v1";

const POLL_SECS: f64 = 4.0;
const POLL_TRIES: u32 = 150;
/// The largest upscaled picture (25.3 MP).
const MAX_UPSCALED: f64 = 25_300_000.0;

fn headers(key: &str) -> Vec<(String, String)> {
    vec![("x-magnific-api-key".into(), key.to_string())]
}

/// Our shape (`"16:9"`) as the service's shape names, for the models
/// that name them.
pub fn named_aspect(aspect: &str, model: &str) -> &'static str {
    let wide = if model.starts_with("seedream") { "cinematic_21_9" } else { "film_horizontal_21_9" };
    match aspect {
        "4:3" => "classic_4_3",
        "3:4" => "traditional_3_4",
        "16:9" => "widescreen_16_9",
        "9:16" => "social_story_9_16",
        "3:2" => "standard_3_2",
        "2:3" => "portrait_2_3",
        "2:1" => "horizontal_2_1",
        "1:2" => "vertical_1_2",
        "5:4" => "social_5_4",
        "4:5" => "social_post_4_5",
        "21:9" => wide,
        _ => "square_1_1",
    }
}

/// The endpoint path (after `/v1`) of a model, for making and for
/// changing a picture.
fn model_path(model: &str, remix: bool) -> Result<&'static str, String> {
    Ok(match (model, remix) {
        ("mystic", _) => "/ai/mystic",
        ("nano-banana-2-1", _) => "/ai/text-to-image/nano-banana-2-1",
        ("gpt-image-2-5", false) => "/ai/text-to-image/gpt-image-2-5",
        ("gpt-image-2-5", true) => "/ai/text-to-image/gpt-image-2-5-edit",
        ("seedream-v5-pro", false) => "/ai/text-to-image/seedream-v5-pro",
        ("seedream-v5-pro", true) => "/ai/text-to-image/seedream-v5-pro-edit",
        ("ideogram-4-5", _) => "/ai/text-to-image/ideogram-4-5",
        ("recraft-v4-1", false) => "/ai/text-to-image/recraft-v4-1",
        ("recraft-v4-1", true) => return Err(tr("studio.model_no_remix")),
        _ => return Err(crate::i18n::trf("studio.unknown_model", &[("name", model)])),
    })
}

fn b64_png(req: &OpRequest, max_pixels: u64) -> Result<(String, u32, u32), String> {
    let png = req.image.as_deref().ok_or_else(|| tr("aiimg.need_bitmap"))?;
    let (b, w, h) = fit_image(png, max_pixels, 0, 256)?;
    Ok((transport::base64(&b), w, h))
}

/// The path and body of the request that starts `req`.
pub fn build_body(req: &OpRequest) -> Result<(&'static str, Value), String> {
    let lower = req.resolution.to_lowercase();
    Ok(match req.op() {
        Op::Generate | Op::Remix => {
            let remix = req.op() == Op::Remix;
            let path = model_path(&req.model, remix)?;
            let mut body = json!({"prompt": req.prompt});
            let image = if remix { Some(b64_png(req, 16_000_000)?.0) } else { None };
            match req.model.as_str() {
                "mystic" => {
                    body["resolution"] = json!(lower);
                    body["aspect_ratio"] = json!(named_aspect(&req.aspect, "mystic"));
                    body["model"] = json!(req.text("engine_model", "realism"));
                    if let Some(img) = image {
                        body["structure_reference"] = json!(img);
                        body["structure_strength"] = json!(req.int("structure", 50).clamp(0, 100));
                    }
                }
                "nano-banana-2-1" => {
                    body["resolution"] = json!(req.resolution);
                    body["aspect_ratio"] = json!(req.aspect);
                    if let Some(img) = image {
                        body["reference_images"] = json!([{"image": img, "mime_type": "image/png"}]);
                    }
                }
                "gpt-image-2-5" => {
                    body["resolution"] = json!(lower);
                    body["aspect_ratio"] = json!(named_aspect(&req.aspect, "gpt"));
                    body["variant"] = json!(req.text("variant", "flare"));
                    body["quality"] = json!(req.text("quality", "high"));
                    body["output_format"] = json!("png");
                    if let Some(img) = image {
                        body["reference_images"] = json!([img]);
                    }
                }
                "seedream-v5-pro" => {
                    body["resolution"] = json!(if lower == "1k" { "1.5k".to_string() } else { "2k".to_string() });
                    body["aspect_ratio"] = json!(named_aspect(&req.aspect, "seedream"));
                    if let Some(img) = image {
                        body["reference_images"] = json!([img]);
                    }
                }
                "ideogram-4-5" => {
                    body["aspect_ratio"] = json!(req.aspect);
                    body["quality"] = json!(req.text("quality", "high"));
                    if let Some(img) = image {
                        body["images"] = json!([img]);
                    }
                }
                "recraft-v4-1" => {
                    body["aspect_ratio"] = json!(named_aspect(&req.aspect, "recraft"));
                    body["tier"] = json!(if lower == "1k" { "mid" } else { "pro" });
                }
                _ => {}
            }
            (path, body)
        }
        Op::Upscale => {
            if req.variant == "precision" {
                let (img, _, _) = b64_png(req, 16_000_000)?;
                (
                    "/ai/image-upscaler-precision-v2",
                    json!({
                        "image": img,
                        "scale_factor": req.int("factor", 2).clamp(2, 16),
                        "flavor": req.text("flavor", "sublime"),
                    }),
                )
            } else {
                let factor = req.int("factor", 2).clamp(2, 16);
                let factor = [2, 4, 8, 16].into_iter().find(|f| *f >= factor).unwrap_or(16);
                // The result may not pass 25.3 MP: the picture sent is made
                // small enough for the factor.
                let max_in = (MAX_UPSCALED / (factor * factor) as f64) as u64;
                let (img, _, _) = b64_png(req, max_in)?;
                let mut body = json!({
                    "image": img,
                    "scale_factor": format!("{factor}x"),
                    "optimized_for": req.text("optimized_for", "standard"),
                    "creativity": req.int("creativity", 0).clamp(-10, 10),
                    "resemblance": req.int("resemblance", 0).clamp(-10, 10),
                });
                if !req.prompt.trim().is_empty() {
                    body["prompt"] = json!(req.prompt);
                }
                ("/ai/image-upscaler", body)
            }
        }
        Op::Expand => {
            let side = |n: &str| req.num(n, 0.0).clamp(0.0, 2048.0).round() as i64;
            let flux = req.variant == "flux";
            let (img, _, _) = b64_png(req, if flux { 20_000_000 } else { 16_000_000 })?;
            let mut body = json!({
                "image": img,
                "left": side("left"),
                "right": side("right"),
                "top": side("top"),
                "bottom": side("bottom"),
            });
            if !req.prompt.trim().is_empty() {
                body["prompt"] = json!(req.prompt);
            }
            (
                if flux { "/ai/image-expand/flux-pro" } else { "/ai/image-expand/seedream-v4-5" },
                body,
            )
        }
        Op::Relight | Op::ReplaceBackground => {
            let (img, _, _) = b64_png(req, 16_000_000)?;
            (
                "/ai/image-relight",
                json!({
                    "image": img,
                    "prompt": req.prompt,
                    "change_background": req.op() == Op::ReplaceBackground,
                    "style": req.text("style", "standard"),
                    "preserve_details": true,
                }),
            )
        }
        Op::RemoveBackground | Op::Vectorize => return Err(tr("studio.not_offered")),
    })
}

/// The task of an answer: its id, status and links.
fn task(v: &Value) -> (String, String, Vec<String>) {
    let d = v.get("data").unwrap_or(v);
    let id = d.get("task_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let status = d
        .get("status")
        .or_else(|| d.get("task_status"))
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string();
    let links = d
        .get("generated")
        .and_then(|g| g.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    (id, status, links)
}

fn download_all(t: &dyn Transport, links: &[String]) -> Result<Output, String> {
    let files = links
        .iter()
        .map(|l| t.download(l, &[]))
        .collect::<Result<Vec<_>, _>>()?;
    if files.is_empty() {
        return Err(tr("aiimg.no_image"));
    }
    Ok(Output::Images(files))
}

/// Upload a picture and give back a link the service can read
/// (`uploads/request-url`, then a PUT with the headers it names).
fn upload(t: &dyn Transport, key: &str, png: &[u8]) -> Result<String, String> {
    let v = transport::json_ok(t.call(&transport::post_json(
        format!("{BASE}/ai/uploads/request-url"),
        headers(key),
        &json!({"files": [{"content_type": "image/png"}]}),
    )))?;
    let f = v
        .pointer("/data/0")
        .or_else(|| v.pointer("/files/0"))
        .or_else(|| v.pointer("/0"))
        .ok_or_else(|| tr("studio.bad_reply"))?;
    let put_url = f.get("upload_url").and_then(|u| u.as_str()).ok_or_else(|| tr("studio.bad_reply"))?;
    let asset = f.get("asset_url").and_then(|u| u.as_str()).ok_or_else(|| tr("studio.bad_reply"))?;
    let mut h: Vec<(String, String)> = f
        .get("headers")
        .and_then(|h| h.as_object())
        .map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
        .unwrap_or_default();
    if !h.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
        h.push(("content-type".into(), "image/png".into()));
    }
    let r = t.call(&crate::net::HttpRequest {
        method: "PUT",
        url: put_url.to_string(),
        headers: h,
        body: String::new(),
        bytes: png.to_vec(),
    })?;
    if !(200..300).contains(&r.status) {
        return Err(transport::service_error(r.status, None, &r.body));
    }
    Ok(asset.to_string())
}

pub fn run(t: &dyn Transport, key: &str, req: &OpRequest) -> Result<Output, String> {
    if req.op() == Op::RemoveBackground {
        let png = req.image.as_deref().ok_or_else(|| tr("aiimg.need_bitmap"))?;
        let (b, _, _) = fit_image(png, 25_000_000, 0, 0)?;
        let link = upload(t, key, &b)?;
        let mut h = headers(key);
        h.push(("content-type".into(), "application/x-www-form-urlencoded".into()));
        let body = format!("image_url={}", urlencode(&link));
        let v = transport::json_ok(t.call(&crate::net::HttpRequest {
            method: "POST",
            url: format!("{BASE}/ai/beta/remove-background"),
            headers: h,
            body,
            bytes: Vec::new(),
        }))?;
        let link = v
            .get("high_resolution")
            .or_else(|| v.get("url"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| tr("studio.bad_reply"))?;
        return download_all(t, &[link.to_string()]);
    }
    let (path, body) = build_body(req)?;
    let url = format!("{BASE}{path}");
    let v = transport::json_ok(t.call(&transport::post_json(url.clone(), headers(key), &body)))?;
    let (id, status, links) = task(&v);
    if status == "COMPLETED" && !links.is_empty() {
        return download_all(t, &links);
    }
    if id.is_empty() {
        return Err(tr("studio.bad_reply"));
    }
    for _ in 0..POLL_TRIES {
        t.wait(POLL_SECS);
        let v = transport::json_ok(t.call(&transport::get(format!("{url}/{id}"), headers(key))))?;
        let (_, status, links) = task(&v);
        match status.as_str() {
            "COMPLETED" => return download_all(t, &links),
            "FAILED" => return Err(tr("studio.task_failed")),
            _ => continue,
        }
    }
    Err(tr("studio.timed_out"))
}

/// Percent-encoding for a form value.
fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The key lists the account's Mystic tasks (`GET /v1/ai/mystic`), which
/// makes nothing.
pub fn test_key(t: &dyn Transport, key: &str) -> Result<String, String> {
    transport::json_ok(t.call(&transport::get(format!("{BASE}/ai/mystic"), headers(key))))?;
    Ok(tr("studio.key_works"))
}

#[cfg(test)]
mod tests {
    use super::super::transport::{reply, Fake};
    use super::*;

    #[test]
    fn models_get_their_own_paths_and_shape_names() {
        let req = OpRequest {
            op: Some(Op::Generate),
            model: "mystic".into(),
            prompt: "a fox".into(),
            aspect: "16:9".into(),
            resolution: "4K".into(),
            ..Default::default()
        };
        let (path, body) = build_body(&req).expect("body");
        assert_eq!(path, "/ai/mystic");
        assert_eq!((body["aspect_ratio"].as_str(), body["resolution"].as_str()), (Some("widescreen_16_9"), Some("4k")));
        let nb = OpRequest { model: "nano-banana-2-1".into(), ..req.clone() };
        let (_, body) = build_body(&nb).expect("body");
        assert_eq!((body["aspect_ratio"].as_str(), body["resolution"].as_str()), (Some("16:9"), Some("4K")));
        let gpt = OpRequest { op: Some(Op::Remix), model: "gpt-image-2-5".into(), image: Some(super::super::test_png(300, 300)), ..req };
        let (path, body) = build_body(&gpt).expect("body");
        assert!(path.ends_with("gpt-image-2-5-edit") && body["reference_images"].is_array());
    }

    #[test]
    fn upscales_keep_the_result_under_the_services_limit() {
        let req = OpRequest {
            op: Some(Op::Upscale),
            variant: "creative".into(),
            image: Some(super::super::test_png(3000, 2000)),
            params: [("factor".to_string(), json!(4))].into_iter().collect(),
            ..Default::default()
        };
        let (path, body) = build_body(&req).expect("body");
        assert_eq!((path, body["scale_factor"].as_str()), ("/ai/image-upscaler", Some("4x")));
        let png = transport::unbase64(body["image"].as_str().expect("image")).expect("b64");
        let (w, h) = super::super::image_size(&png).expect("size");
        assert!((w as f64 * h as f64) * 16.0 <= MAX_UPSCALED + 1.0);
    }

    #[test]
    fn a_task_is_followed_to_its_picture_and_downloaded() {
        fn answer(r: &crate::net::HttpRequest) -> crate::net::HttpResult {
            if r.method == "POST" {
                return reply(200, json!({"data": {"task_id": "t1", "status": "CREATED", "generated": []}}));
            }
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if N.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                reply(200, json!({"data": {"task_id": "t1", "status": "IN_PROGRESS", "generated": []}}))
            } else {
                reply(200, json!({"data": {"task_id": "t1", "status": "COMPLETED", "generated": ["https://cdn.example/x.png"]}}))
            }
        }
        let fake = Fake::new(answer, |url| {
            assert_eq!(url, "https://cdn.example/x.png");
            Ok(super::super::test_png(8, 8))
        });
        let req = OpRequest {
            op: Some(Op::Relight),
            prompt: "warm sunset light".into(),
            image: Some(super::super::test_png(300, 300)),
            ..Default::default()
        };
        assert!(matches!(run(&fake, "mk", &req), Ok(Output::Images(v)) if v.len() == 1));
        let sent = fake.sent();
        assert!(sent[0].headers.contains(&("x-magnific-api-key".into(), "mk".into())));
        assert!(sent.last().is_some_and(|r| r.url.ends_with("/ai/image-relight/t1")));
    }

    #[test]
    fn background_removal_uploads_the_picture_first() {
        fn answer(r: &crate::net::HttpRequest) -> crate::net::HttpResult {
            if r.url.ends_with("/uploads/request-url") {
                reply(200, json!({"data": [{"file_id": "upl_img_1", "upload_url": "https://up.example/put", "headers": {"Content-Type": "image/png", "x-goog-content-length-range": "0,1000000"}, "asset_url": "https://up.example/a.png?sig=1"}]}))
            } else if r.method == "PUT" {
                reply(200, json!({}))
            } else {
                reply(200, json!({"high_resolution": "https://cdn.example/cut.png", "url": "https://cdn.example/cut.png"}))
            }
        }
        let fake = Fake::new(answer, |_| Ok(super::super::test_png(8, 8)));
        let req = OpRequest { op: Some(Op::RemoveBackground), image: Some(super::super::test_png(300, 300)), ..Default::default() };
        assert!(run(&fake, "mk", &req).is_ok());
        let sent = fake.sent();
        assert_eq!(sent[1].method, "PUT");
        assert!(!sent[1].headers.iter().any(|(k, _)| k == "x-magnific-api-key"), "no key on the upload");
        assert!(sent[2].body.starts_with("image_url=https%3A%2F%2Fup.example"));
    }
}
