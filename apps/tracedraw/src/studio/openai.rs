//! The studio's adapter for OpenAI's GPT Image models
//! (developers.openai.com, read 2026-10-23): `https://api.openai.com/v1`,
//! `Authorization: Bearer`, `images/generations` and `images/edits` (JSON,
//! pictures as data URLs in `images`, the answer always `b64_json`).
//! GPT Image 2 and 2.5 take any size whose sides are multiples of 16,
//! 1:3 to 3:1, at most 3840 a side and 655,360 to 8,294,400 pixels, so the
//! studio asks for the exact shape at the chosen size. Background removal
//! is an edit with a transparent background; expanding is an edit of the
//! picture on a larger canvas with a mask over the new part.

use serde_json::{json, Value};

use super::transport::{self, Transport};
use super::{fit_image, Op, OpRequest, Output};
use crate::i18n::tr;

pub const BASE: &str = "https://api.openai.com/v1";

const MIN_PIXELS: f64 = 655_360.0;
const MAX_PIXELS: f64 = 8_294_400.0;
const MAX_SIDE: f64 = 3840.0;

fn headers(key: &str) -> Vec<(String, String)> {
    vec![("authorization".into(), format!("Bearer {key}"))]
}

/// Width over height of `"16:9"`.
fn ratio(aspect: &str) -> f64 {
    let mut it = aspect.split(':').map(|x| x.trim().parse::<f64>().unwrap_or(1.0));
    let (w, h) = (it.next().unwrap_or(1.0), it.next().unwrap_or(1.0));
    (w / h.max(0.01)).clamp(1.0 / 3.0, 3.0)
}

/// A size the API takes, as near as can be to `w` by `h`: sides rounded
/// to 16, inside the pixel and side limits.
pub fn legal_size(w: f64, h: f64) -> (u32, u32) {
    let r = (w / h.max(1.0)).clamp(1.0 / 3.0, 3.0);
    let mut px = (w * h).clamp(MIN_PIXELS, MAX_PIXELS);
    let mut cw = (px * r).sqrt();
    let mut ch = (px / r).sqrt();
    if cw > MAX_SIDE || ch > MAX_SIDE {
        let k = MAX_SIDE / cw.max(ch);
        cw *= k;
        ch *= k;
        px = cw * ch;
    }
    let _ = px;
    let round = |x: f64| ((x / 16.0).floor() * 16.0).max(16.0) as u32;
    let (mut a, mut b) = (round(cw), round(ch));
    while (a as f64) * (b as f64) < MIN_PIXELS {
        a += 16;
        b = ((a as f64 / r / 16.0).round() * 16.0) as u32;
    }
    (a, b)
}

/// The size for a shape at `"1K"`, `"2K"` or `"4K"` (about 1, 4 and
/// 8.3 megapixels, the last being the most the API makes).
pub fn size_for(aspect: &str, resolution: &str) -> (u32, u32) {
    let target = match resolution {
        "4K" => MAX_PIXELS,
        "2K" => 2048.0 * 2048.0,
        _ => 1024.0 * 1024.0,
    };
    let r = ratio(aspect);
    legal_size((target * r).sqrt(), (target / r).sqrt())
}

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", transport::base64(png))
}

pub fn build(key: &str, req: &OpRequest) -> Result<crate::net::HttpRequest, String> {
    let model = if req.model.is_empty() {
        req.text("model", "gpt-image-2.5-sunburst").to_string()
    } else {
        req.model.clone()
    };
    let quality = req.text("quality", "high").to_string();
    if req.op() == Op::Generate {
        let (w, h) = size_for(&req.aspect, &req.resolution);
        let body = json!({
            "model": model,
            "prompt": req.prompt,
            "n": 1,
            "size": format!("{w}x{h}"),
            "quality": quality,
            "output_format": "png",
        });
        return Ok(transport::post_json(format!("{BASE}/images/generations"), headers(key), &body));
    }
    let png = req.image.as_deref().ok_or_else(|| tr("aiimg.need_bitmap"))?;
    let (img, w, h) = fit_image(png, MAX_PIXELS as u64, MAX_SIDE as u32, 0)?;
    let mut body = json!({"model": model, "n": 1, "quality": quality, "output_format": "png"});
    let prompt = match req.op() {
        Op::Remix => req.prompt.clone(),
        Op::RemoveBackground => {
            body["background"] = json!("transparent");
            "Remove the background completely and keep only the main subject, unchanged, at the same size and position, on a transparent background.".to_string()
        }
        Op::ReplaceBackground => format!(
            "Keep the main subject exactly as it is, same pose, position, size and details. Replace only the background with: {}",
            req.prompt
        ),
        Op::Relight => format!(
            "Keep everything exactly as it is, same composition and details. Change only the lighting: {}",
            req.prompt
        ),
        Op::Expand => {
            // The picture on a larger canvas, the new part transparent in
            // both the picture and the mask, which the edit fills.
            let side = |n: &str| req.num(n, 0.0).max(0.0) as u32;
            let (l, r, t, b) = (side("left"), side("right"), side("top"), side("bottom"));
            let (cw, ch) = legal_size((w + l + r) as f64, (h + t + b) as f64);
            let k = cw as f64 / (w + l + r) as f64;
            let canvas = expand_canvas(&img, cw, ch, (l as f64 * k) as u32, (t as f64 * k) as u32, k)?;
            body["size"] = json!(format!("{cw}x{ch}"));
            body["images"] = json!([{"image_url": data_url(&canvas)}]);
            body["mask"] = json!({"image_url": data_url(&canvas)});
            body["prompt"] = json!(if req.prompt.trim().is_empty() {
                "Extend the picture naturally into the transparent area, matching its style, light and content. Do not change the existing part.".to_string()
            } else {
                format!("Extend the picture into the transparent area: {}. Do not change the existing part.", req.prompt)
            });
            return Ok(transport::post_json(format!("{BASE}/images/edits"), headers(key), &body));
        }
        Op::Upscale => {
            let (uw, uh) = legal_size(w as f64 * 2.0, h as f64 * 2.0);
            body["size"] = json!(format!("{uw}x{uh}"));
            "Recreate this exact picture at a higher resolution with finer detail. Do not change the composition, colours, shapes, text or style.".to_string()
        }
        Op::Vectorize | Op::Generate => return Err(tr("studio.not_offered")),
    };
    if body.get("size").is_none() {
        let (sw, sh) = legal_size(w as f64, h as f64);
        body["size"] = json!(format!("{sw}x{sh}"));
    }
    body["prompt"] = json!(prompt);
    body["images"] = json!([{"image_url": data_url(&img)}]);
    Ok(transport::post_json(format!("{BASE}/images/edits"), headers(key), &body))
}

/// The picture `png` scaled by `k` and placed at (`x`, `y`) on a
/// transparent canvas of `cw` by `ch`.
fn expand_canvas(png: &[u8], cw: u32, ch: u32, x: u32, y: u32, k: f64) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(png)
        .map_err(|_| tr("aiimg.unreadable_image"))?
        .to_rgba8();
    let nw = ((img.width() as f64 * k).round() as u32).max(1);
    let nh = ((img.height() as f64 * k).round() as u32).max(1);
    let scaled = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Lanczos3);
    let mut canvas = image::RgbaImage::new(cw, ch);
    image::imageops::overlay(&mut canvas, &scaled, x as i64, y as i64);
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn run(t: &dyn Transport, key: &str, req: &OpRequest) -> Result<Output, String> {
    let http = build(key, req)?;
    let v: Value = transport::json_ok(t.call(&http))?;
    let files: Vec<Vec<u8>> = v
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|i| i.get("b64_json").and_then(|b| b.as_str()))
                .filter_map(transport::unbase64)
                .collect()
        })
        .unwrap_or_default();
    if files.is_empty() {
        return Err(tr("aiimg.no_image"));
    }
    Ok(Output::Images(files))
}

/// The key lists the models (`GET /v1/models`).
pub fn test_key(t: &dyn Transport, key: &str) -> Result<String, String> {
    let v = transport::json_ok(t.call(&transport::get(format!("{BASE}/models"), headers(key))))?;
    let n = v.get("data").and_then(|d| d.as_array()).map_or(0, |d| d.len());
    Ok(crate::i18n::trf("studio.models_listed", &[("n", &n.to_string())]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_exact_shapes_the_api_takes() {
        for (aspect, res) in [("16:9", "1K"), ("16:9", "4K"), ("2:3", "2K"), ("1:1", "4K"), ("21:9", "2K"), ("1:1", "1K")] {
            let (w, h) = size_for(aspect, res);
            assert!(w % 16 == 0 && h % 16 == 0, "{aspect} {res}");
            let px = w as f64 * h as f64;
            assert!((MIN_PIXELS..=MAX_PIXELS).contains(&px), "{aspect} {res}: {w}x{h}");
            assert!(w as f64 <= MAX_SIDE && h as f64 <= MAX_SIDE);
            let r = w as f64 / h as f64;
            assert!((r - ratio(aspect)).abs() / ratio(aspect) < 0.03, "{aspect} {res}: {w}x{h}");
        }
        assert_eq!(size_for("1:1", "1K"), (1024, 1024));
        assert_eq!(size_for("16:9", "4K"), (3840, 2160));
    }

    #[test]
    fn edits_send_the_picture_inline_and_expanding_adds_a_mask() {
        let req = OpRequest {
            op: Some(Op::RemoveBackground),
            model: "gpt-image-2.5-sunburst".into(),
            image: Some(super::super::test_png(800, 600)),
            ..Default::default()
        };
        let r = build("ok", &req).expect("request");
        assert_eq!(r.url, "https://api.openai.com/v1/images/edits");
        let v: Value = serde_json::from_str(&r.body).expect("json");
        assert_eq!(v["background"], "transparent");
        assert!(v.pointer("/images/0/image_url").and_then(|u| u.as_str()).is_some_and(|u| u.starts_with("data:image/png")));
        let ex = OpRequest {
            op: Some(Op::Expand),
            params: [("right".to_string(), json!(400))].into_iter().collect(),
            ..req
        };
        let v: Value = serde_json::from_str(&build("ok", &ex).expect("request").body).expect("json");
        assert!(v.get("mask").is_some());
        let size = v["size"].as_str().expect("size");
        let (w, h) = size.split_once('x').map(|(a, b)| (a.parse::<f64>().unwrap_or(0.0), b.parse::<f64>().unwrap_or(1.0))).expect("WxH");
        assert!((w / h - 1200.0 / 600.0).abs() < 0.05, "{size}");
    }
}
