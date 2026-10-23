//! The studio's adapter for Recraft (recraft.ai/docs/api-reference, read
//! 2026-10-23): `https://external.api.recraft.ai/v1`, `Authorization:
//! Bearer`, JSON bodies with pictures as data URLs and answers asked for
//! as `b64_json`; every request answers when its picture is ready.
//! Generation and image-to-image answer `data[]`; the tools (vectorize,
//! remove background, upscales, enhance) answer one `image`. Models
//! ending in `_vector` make SVG.

use serde_json::{json, Value};

use super::transport::{self, Transport};
use super::{fit_image, Op, OpRequest, Output};
use crate::i18n::tr;

pub const BASE: &str = "https://external.api.recraft.ai/v1";

/// Input limits: 16 MP, 4096 px a side, 256 px the shorter side.
const MAX_PIXELS: u64 = 16_000_000;
const MAX_SIDE: u32 = 4096;
const MIN_SIDE: u32 = 256;

fn headers(key: &str) -> Vec<(String, String)> {
    vec![("authorization".into(), format!("Bearer {key}"))]
}

fn data_url(req: &OpRequest, min_side: u32) -> Result<String, String> {
    let png = req.image.as_deref().ok_or_else(|| tr("aiimg.need_bitmap"))?;
    let (b, _, _) = fit_image(png, MAX_PIXELS, MAX_SIDE, min_side)?;
    Ok(format!("data:image/png;base64,{}", transport::base64(&b)))
}

/// The model makes SVG.
pub fn is_vector(model: &str) -> bool {
    model.ends_with("_vector")
}

/// The request for `req`, and whether its answer is one `image` (the
/// tools) rather than `data[]`.
pub fn build(key: &str, req: &OpRequest) -> Result<(crate::net::HttpRequest, bool), String> {
    let mut body = json!({"response_format": "b64_json"});
    let (path, single) = match req.op() {
        Op::Generate => {
            body["prompt"] = json!(req.prompt);
            body["model"] = json!(req.model);
            body["n"] = json!(1);
            body["size"] = json!(req.aspect);
            if !is_vector(&req.model) {
                body["image_format"] = json!("png");
            }
            ("/images/generations", false)
        }
        Op::Remix => {
            body["image_url"] = json!(data_url(req, MIN_SIDE)?);
            body["prompt"] = json!(req.prompt);
            body["strength"] = json!(req.num("strength", 0.5).clamp(0.0, 1.0));
            if req.model.contains("flash") {
                return Err(tr("studio.model_no_remix"));
            }
            body["model"] = json!(req.model);
            if !is_vector(&req.model) {
                body["image_format"] = json!("png");
            }
            ("/images/imageToImage", false)
        }
        Op::Upscale => {
            let path = match req.variant.as_str() {
                "creative" => "/images/creativeUpscale",
                "enhance" => {
                    body["refinement"] = json!(req.text("refinement", "moderate"));
                    "/images/enhance"
                }
                _ => "/images/crispUpscale",
            };
            let min = if req.variant == "crisp" || req.variant.is_empty() { 32 } else { MIN_SIDE };
            body["image_url"] = json!(data_url(req, min)?);
            body["image_format"] = json!("png");
            (path, true)
        }
        Op::Expand => {
            body["image_url"] = json!(data_url(req, MIN_SIDE)?);
            let prompt = if req.prompt.trim().is_empty() {
                "continue the picture naturally beyond its edges".to_string()
            } else {
                req.prompt.clone()
            };
            body["prompt"] = json!(prompt);
            for (ours, theirs) in [("left", "expand_left"), ("right", "expand_right"), ("top", "expand_top"), ("bottom", "expand_bottom")] {
                let v = req.num(ours, 0.0).clamp(0.0, 4096.0).round() as u64;
                if v > 0 {
                    body[theirs] = json!(v);
                }
            }
            body["image_format"] = json!("png");
            ("/images/outpaint", false)
        }
        Op::RemoveBackground => {
            body["image_url"] = json!(data_url(req, MIN_SIDE)?);
            body["image_format"] = json!("png");
            ("/images/removeBackground", true)
        }
        Op::ReplaceBackground => {
            body["image_url"] = json!(data_url(req, MIN_SIDE)?);
            body["prompt"] = json!(req.prompt);
            body["image_format"] = json!("png");
            ("/images/replaceBackground", false)
        }
        Op::Vectorize => {
            body["image_url"] = json!(data_url(req, MIN_SIDE)?);
            body["shape_stacking"] = json!(req.text("stacking", "hierarchical"));
            ("/images/vectorize", true)
        }
        Op::Relight => return Err(tr("studio.not_offered")),
    };
    Ok((transport::post_json(format!("{BASE}{path}"), headers(key), &body), single))
}

/// The picture or SVG of an answer.
fn output(v: &Value, single: bool, vector: bool) -> Result<Output, String> {
    let b64 = if single {
        v.pointer("/image/b64_json")
    } else {
        v.pointer("/data/0/b64_json")
    }
    .and_then(|b| b.as_str())
    .ok_or_else(|| tr("studio.bad_reply"))?;
    let bytes = transport::unbase64(b64).ok_or_else(|| tr("studio.bad_reply"))?;
    let looks_svg = bytes.windows(4).take(512).any(|w| w == b"<svg");
    if vector || looks_svg {
        return String::from_utf8(bytes)
            .map(Output::Svg)
            .map_err(|_| tr("aiimg.no_svg"));
    }
    Ok(Output::Images(vec![bytes]))
}

pub fn run(t: &dyn Transport, key: &str, req: &OpRequest) -> Result<Output, String> {
    let (http, single) = build(key, req)?;
    let v = transport::json_ok(t.call(&http))?;
    let vector = req.op() == Op::Vectorize
        || (matches!(req.op(), Op::Generate | Op::Remix) && is_vector(&req.model));
    output(&v, single, vector)
}

/// The account's balance (`GET /v1/users/me`, `credits` in units of a
/// thousandth of a dollar).
pub fn test_key(t: &dyn Transport, key: &str) -> Result<String, String> {
    let v = transport::json_ok(t.call(&transport::get(format!("{BASE}/users/me"), headers(key))))?;
    let units = v.get("credits").and_then(|c| c.as_f64()).unwrap_or(0.0);
    Ok(crate::i18n::trf(
        "studio.balance_usd",
        &[("amount", &super::catalog::dollars(units / 1000.0))],
    ))
}

#[cfg(test)]
mod tests {
    use super::super::transport::{reply, Fake};
    use super::*;

    #[test]
    fn requests_are_json_with_the_picture_inline() {
        let req = OpRequest {
            op: Some(Op::Generate),
            model: "recraftv4_1_vector".into(),
            prompt: "a fox".into(),
            aspect: "16:9".into(),
            ..Default::default()
        };
        let (r, single) = build("t", &req).expect("request");
        assert_eq!(r.url, "https://external.api.recraft.ai/v1/images/generations");
        assert!(!single);
        let v: Value = serde_json::from_str(&r.body).expect("json");
        assert_eq!((v["model"].as_str(), v["size"].as_str()), (Some("recraftv4_1_vector"), Some("16:9")));
        assert!(v.get("image_format").is_none(), "vectors take no image format");
        let up = OpRequest {
            op: Some(Op::Upscale),
            variant: "crisp".into(),
            image: Some(super::super::test_png(100, 80)),
            ..Default::default()
        };
        let (r, single) = build("t", &up).expect("request");
        assert!(r.url.ends_with("/crispUpscale") && single);
        let v: Value = serde_json::from_str(&r.body).expect("json");
        assert!(v["image_url"].as_str().is_some_and(|u| u.starts_with("data:image/png;base64,")));
        let ex = OpRequest {
            op: Some(Op::Expand),
            image: Some(super::super::test_png(300, 300)),
            params: [("top".to_string(), json!(120))].into_iter().collect(),
            ..Default::default()
        };
        let v: Value = serde_json::from_str(&build("t", &ex).expect("request").0.body).expect("json");
        assert_eq!(v["expand_top"], 120);
        assert!(v.get("expand_left").is_none());
    }

    #[test]
    fn vectors_come_back_as_svg_and_pictures_as_bytes() {
        let svg = transport::base64(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
        let fake = Fake::new(
            |r| {
                if r.url.ends_with("/vectorize") {
                    reply(200, json!({"image": {"image_id": "1", "b64_json": transport::base64(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>")}}))
                } else {
                    reply(200, json!({"data": [{"image_id": "1", "b64_json": transport::base64(&super::super::test_png(4, 4))}]}))
                }
            },
            |_| Err("no".into()),
        );
        let _ = svg;
        let vec = OpRequest { op: Some(Op::Vectorize), image: Some(super::super::test_png(300, 300)), ..Default::default() };
        assert!(matches!(run(&fake, "t", &vec).expect("svg"), Output::Svg(s) if s.contains("<svg")));
        let gen = OpRequest { op: Some(Op::Generate), model: "recraftv4_1".into(), prompt: "x".into(), aspect: "1:1".into(), ..Default::default() };
        assert!(matches!(run(&fake, "t", &gen).expect("png"), Output::Images(v) if v.len() == 1));
        let fake = Fake::new(|_| reply(200, json!({"id": "u", "credits": 1250})), |_| Err("no".into()));
        assert!(test_key(&fake, "t").expect("works").contains("1,25") || test_key(&fake, "t").expect("works").contains("1.25"));
    }
}
