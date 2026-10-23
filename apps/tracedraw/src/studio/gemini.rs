//! The studio's adapter for Google Gemini's image models
//! (ai.google.dev/gemini-api, read 2026-10-23): `generateContent` at
//! `https://generativelanguage.googleapis.com/v1beta/models/{model}`,
//! `x-goog-api-key`, the picture as `inline_data` and the answer's
//! pictures in `inlineData`. `imageConfig` names the shape and the size
//! (`1K`, `2K`, `4K`, upper-case K). Gemini has no tool for backgrounds,
//! light or enlarging; those are asked for in words with the picture
//! (the catalog says so), and a background to remove is asked for as
//! flat green, keyed out by the studio.

use serde_json::{json, Value};

use super::transport::{self, Transport};
use super::{fit_image, Op, OpRequest, Output};
use crate::i18n::tr;

pub const BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Pictures sent are kept under this many pixels (the models take
/// large inputs, but a page's bitmap can be huge).
const MAX_PIXELS: u64 = 16_000_000;

fn headers(key: &str) -> Vec<(String, String)> {
    vec![("x-goog-api-key".into(), key.to_string())]
}

/// The words sent with the picture for each edit.
fn edit_words(req: &OpRequest) -> Result<String, String> {
    Ok(match req.op() {
        Op::Remix => req.prompt.clone(),
        Op::RemoveBackground => crate::ai_image::CHROMA_PROMPT.to_string(),
        Op::ReplaceBackground => format!(
            "Keep the main subject exactly as it is, same pose, position, size, lighting on it and details. Replace only the background with: {}",
            req.prompt
        ),
        Op::Relight => format!(
            "Keep everything in the picture exactly as it is, same composition and details. Change only the lighting: {}",
            req.prompt
        ),
        Op::Upscale => "Recreate this exact picture at a higher resolution with finer detail. Do not change the composition, colours, shapes, text or style.".to_string(),
        Op::Expand | Op::Vectorize | Op::Generate => return Err(tr("studio.not_offered")),
    })
}

pub fn build(key: &str, req: &OpRequest) -> Result<crate::net::HttpRequest, String> {
    let mut parts = Vec::new();
    let text = if req.op() == Op::Generate {
        req.prompt.clone()
    } else {
        let png = req.image.as_deref().ok_or_else(|| tr("aiimg.need_bitmap"))?;
        let (b, _, _) = fit_image(png, MAX_PIXELS, 0, 0)?;
        parts.push(json!({"inline_data": {"mime_type": "image/png", "data": transport::base64(&b)}}));
        edit_words(req)?
    };
    // The text after the picture, as Gemini's own examples have it.
    parts.push(json!({"text": text}));
    let mut config = json!({"responseModalities": ["TEXT", "IMAGE"], "imageConfig": {}});
    if !req.aspect.is_empty() {
        config["imageConfig"]["aspectRatio"] = json!(req.aspect);
    } else if req.image_size.0 > 0 && req.image_size.1 > 0 {
        config["imageConfig"]["aspectRatio"] =
            json!(crate::ai_image::Aspect::nearest(req.image_size.0, req.image_size.1).ratio());
    }
    let res = if req.resolution.is_empty() {
        req.text("resolution", "1K").to_string()
    } else {
        req.resolution.clone()
    };
    config["imageConfig"]["imageSize"] = json!(res);
    let model = if req.model.is_empty() {
        req.text("model", "gemini-nano-banana-2.1").to_string()
    } else {
        req.model.clone()
    };
    let body = json!({
        "contents": [{"role": "user", "parts": parts}],
        "generationConfig": config,
    });
    Ok(transport::post_json(
        format!("{BASE}/models/{model}:generateContent"),
        headers(key),
        &body,
    ))
}

pub fn run(t: &dyn Transport, key: &str, req: &OpRequest) -> Result<Output, String> {
    let http = build(key, req)?;
    let r = t.call(&http)?;
    let files = crate::ai_image::parse_images(crate::ai_image::ImageProvider::Gemini, &r)?;
    Ok(Output::Images(files))
}

/// The key lists the models (`GET /v1beta/models`).
pub fn test_key(t: &dyn Transport, key: &str) -> Result<String, String> {
    let v: Value = transport::json_ok(t.call(&transport::get(format!("{BASE}/models"), headers(key))))?;
    let n = v.get("models").and_then(|m| m.as_array()).map_or(0, |m| m.len());
    Ok(crate::i18n::trf("studio.models_listed", &[("n", &n.to_string())]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_and_words_go_in_one_request_with_shape_and_size() {
        let req = OpRequest {
            op: Some(Op::ReplaceBackground),
            model: "gemini-3-pro-image".into(),
            prompt: "a sunny beach".into(),
            image: Some(super::super::test_png(64, 32)),
            image_size: (64, 32),
            resolution: "4K".into(),
            ..Default::default()
        };
        let r = build("gk", &req).expect("request");
        assert_eq!(
            r.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-pro-image:generateContent"
        );
        assert!(r.headers.contains(&("x-goog-api-key".into(), "gk".into())));
        let v: Value = serde_json::from_str(&r.body).expect("json");
        assert!(v.pointer("/contents/0/parts/0/inline_data/data").is_some());
        assert!(v.pointer("/contents/0/parts/1/text").and_then(|t| t.as_str()).is_some_and(|t| t.contains("sunny beach")));
        assert_eq!(v.pointer("/generationConfig/imageConfig/imageSize"), Some(&json!("4K")));
        assert_eq!(v.pointer("/generationConfig/imageConfig/aspectRatio"), Some(&json!("16:9")));
        let ex = OpRequest { op: Some(Op::Expand), ..req };
        assert!(build("gk", &ex).is_err());
    }
}
