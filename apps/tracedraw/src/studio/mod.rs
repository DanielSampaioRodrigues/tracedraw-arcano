//! AI studio (Arcano edition): several AI services side by side, each
//! with its own key, for pictures and for what is done to them.
//!
//! The upstream editor speaks to one AI service with one key
//! (`ai.rs`, `ai_image.rs`). The studio keeps that service for drawings
//! and adds, for pictures, any of the services in its catalog
//! (`catalog.rs`): Google Gemini, OpenAI, Recraft, Freepik (Magnific) and
//! Stability AI. Each service's key is kept on its own, so switching the
//! picture service, or using one service to make a picture and another to
//! enlarge it, never asks for a key again.
//!
//! Operations ([`Op`]): make a picture, change one (remix), enlarge it
//! (upscale), extend its edges (expand), remove or replace its
//! background, relight it and trace it into vectors. Which service does
//! what, its models, sizes, shapes and listed prices come from the
//! catalog, which can be refreshed from the edition's repository without
//! a new build. The requests themselves are written by one adapter per
//! service (`gemini.rs`, `openai.rs`, `recraft.rs`, `freepik.rs`,
//! `stability.rs`), each a blocking pipeline (send, wait for a task when
//! the service works in the background, download) run on a worker thread
//! through a [`transport::Transport`], which tests replace.

pub mod app;
pub mod catalog;
pub mod freepik;
pub mod gemini;
pub mod openai;
pub mod recraft;
pub mod stability;
pub mod transport;
pub mod ui;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use catalog::{Catalog, ModelInfo, Variant};

/// What is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// A new picture from words.
    Generate,
    /// A picture changed as the words say.
    Remix,
    /// More pixels and detail, same picture.
    Upscale,
    /// The picture extended past its edges.
    Expand,
    /// The subject kept, the rest transparent.
    RemoveBackground,
    /// The subject kept on a new background the words describe.
    ReplaceBackground,
    /// New light on the picture.
    Relight,
    /// The picture traced into vectors (SVG).
    Vectorize,
}

impl Op {
    #[cfg(test)]
    pub const ALL: [Op; 8] = [
        Op::Generate,
        Op::Remix,
        Op::Upscale,
        Op::Expand,
        Op::RemoveBackground,
        Op::ReplaceBackground,
        Op::Relight,
        Op::Vectorize,
    ];

    /// The operations done to a selected bitmap.
    pub const EDITS: [Op; 6] = [
        Op::Upscale,
        Op::Expand,
        Op::RemoveBackground,
        Op::ReplaceBackground,
        Op::Relight,
        Op::Vectorize,
    ];

    /// The catalog's name of the operation.
    pub fn id(self) -> &'static str {
        match self {
            Op::Generate => "generate",
            Op::Remix => "remix",
            Op::Upscale => "upscale",
            Op::Expand => "expand",
            Op::RemoveBackground => "remove_background",
            Op::ReplaceBackground => "replace_background",
            Op::Relight => "relight",
            Op::Vectorize => "vectorize",
        }
    }

    #[cfg(test)]
    pub fn from_id(id: &str) -> Option<Op> {
        Op::ALL.iter().copied().find(|o| o.id() == id)
    }

    /// i18n key of the operation's name.
    pub fn key(self) -> &'static str {
        match self {
            Op::Generate => "studio.op_generate",
            Op::Remix => "studio.op_remix",
            Op::Upscale => "studio.op_upscale",
            Op::Expand => "studio.op_expand",
            Op::RemoveBackground => "studio.op_remove_background",
            Op::ReplaceBackground => "studio.op_replace_background",
            Op::Relight => "studio.op_relight",
            Op::Vectorize => "studio.op_vectorize",
        }
    }

    /// i18n key of a line on what the operation does.
    pub fn about_key(self) -> &'static str {
        match self {
            Op::Generate => "studio.about_generate",
            Op::Remix => "studio.about_remix",
            Op::Upscale => "studio.about_upscale",
            Op::Expand => "studio.about_expand",
            Op::RemoveBackground => "studio.about_remove_background",
            Op::ReplaceBackground => "studio.about_replace_background",
            Op::Relight => "studio.about_relight",
            Op::Vectorize => "studio.about_vectorize",
        }
    }

    /// The operation takes words (a prompt).
    pub fn takes_prompt(self) -> bool {
        matches!(
            self,
            Op::Generate | Op::Remix | Op::ReplaceBackground | Op::Relight
        )
    }

}

/// Everything one request needs. `params` carries what is particular to
/// an operation or a service (`factor`, `left`/`right`/`top`/`bottom` in
/// pixels, `creativity`, `style`...), as the catalog's variants name them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpRequest {
    pub op: Option<Op>,
    pub provider: String,
    /// The model, for operations that take one.
    pub model: String,
    /// The catalog variant of the operation (`crisp`, `creative`...).
    pub variant: String,
    pub prompt: String,
    /// The picture worked on, as PNG.
    pub image: Option<Vec<u8>>,
    /// Its size in pixels.
    pub image_size: (u32, u32),
    /// `"1:1"`, `"16:9"`...
    pub aspect: String,
    /// `"1K"`, `"2K"`, `"4K"`.
    pub resolution: String,
    pub count: u8,
    pub params: BTreeMap<String, serde_json::Value>,
    /// The service takes words in English only: the prompt is translated
    /// first with `translator` (the main AI service and its key), when
    /// there is one.
    pub english: bool,
    pub translator: Option<(crate::ai::AiPrefs, String)>,
}

impl OpRequest {
    pub fn op(&self) -> Op {
        self.op.unwrap_or(Op::Generate)
    }

    /// A number parameter, or `default`.
    pub fn num(&self, name: &str, default: f64) -> f64 {
        self.params
            .get(name)
            .and_then(|v| v.as_f64())
            .unwrap_or(default)
    }

    /// A text parameter, or `default`.
    pub fn text<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.params
            .get(name)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    /// A whole number parameter, or `default`.
    pub fn int(&self, name: &str, default: i64) -> i64 {
        self.params
            .get(name)
            .and_then(|v| v.as_i64())
            .unwrap_or(default)
    }
}

/// What a request gave back.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// Picture files (PNG, JPEG or WebP).
    Images(Vec<Vec<u8>>),
    /// One SVG document.
    Svg(String),
}

/// Run `req` with `key` through the service's adapter. Blocking: it waits
/// for tasks a service runs in the background.
/// What the translation is asked for.
pub const TRANSLATE_PROMPT: &str = "Translate the user's text into natural English for an image generation service. Keep every detail, name and number; do not add anything. Reply with the translation only, without quotes or notes. If it is already in English, reply with it unchanged.";

/// `text` in English, through the main AI service.
pub fn translate(t: &dyn transport::Transport, prefs: &crate::ai::AiPrefs, key: &str, text: &str) -> Result<String, String> {
    let http = crate::ai::build_request_with(
        prefs,
        key,
        TRANSLATE_PROMPT,
        &[(crate::ai::Role::User, text.to_string())],
    );
    let r = t.call(&http)?;
    let out = crate::ai::parse_reply(prefs.provider, &r)?;
    let out = out.trim().trim_matches('"').trim().to_string();
    if out.is_empty() {
        Err(crate::i18n::tr("studio.bad_reply"))
    } else {
        Ok(out)
    }
}

/// Words a person types are not all ASCII when they are not English
/// (accents); plain ASCII is sent as it is.
fn needs_translation(text: &str) -> bool {
    !text.is_ascii()
        || text
            .split(|c: char| !c.is_alphabetic())
            .any(|w| matches!(w.to_lowercase().as_str(), "um" | "uma" | "com" | "de" | "do" | "da" | "em" | "no" | "na" | "para" | "que"))
}

pub fn run(t: &dyn transport::Transport, key: &str, req: &OpRequest) -> Result<Output, String> {
    if key.trim().is_empty() {
        return Err(crate::i18n::tr("studio.no_key"));
    }
    if req.english && needs_translation(&req.prompt) {
        let Some((prefs, tkey)) = &req.translator else {
            return Err(crate::i18n::tr("studio.english_only"));
        };
        let mut r = req.clone();
        r.prompt = translate(t, prefs, tkey, &req.prompt)?;
        r.english = false;
        return run(t, key, &r);
    }
    match req.provider.as_str() {
        "gemini" => gemini::run(t, key, req),
        "openai" => openai::run(t, key, req),
        "recraft" => recraft::run(t, key, req),
        "freepik" => freepik::run(t, key, req),
        "stability" => stability::run(t, key, req),
        other => Err(crate::i18n::trf("studio.unknown_service", &[("name", other)])),
    }
}

/// A cheap request that says whether `key` works for `provider`:
/// `Ok(note)` (a balance or a model count when the service gives one).
pub fn test_key(t: &dyn transport::Transport, provider: &str, key: &str) -> Result<String, String> {
    if key.trim().is_empty() {
        return Err(crate::i18n::tr("studio.no_key"));
    }
    match provider {
        "gemini" => gemini::test_key(t, key),
        "openai" => openai::test_key(t, key),
        "recraft" => recraft::test_key(t, key),
        "freepik" => freepik::test_key(t, key),
        "stability" => stability::test_key(t, key),
        other => Err(crate::i18n::trf("studio.unknown_service", &[("name", other)])),
    }
}

/// The studio's preferences, kept with the others.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StudioPrefs {
    /// The picture service (a catalog provider id). Empty: the one AI
    /// service of Tools > Options > AI, as the upstream editor does.
    pub image_provider: String,
    /// The model chosen for each service.
    pub models: BTreeMap<String, String>,
    /// The service chosen for each edit operation (by `Op::id`).
    pub op_providers: BTreeMap<String, String>,
    /// The variant chosen for each operation of each service
    /// (`"<provider>/<op>"`).
    pub variants: BTreeMap<String, String>,
    /// `"1:1"` and so on.
    pub aspect: String,
    /// `"1K"`, `"2K"`, `"4K"`.
    pub resolution: String,
    /// Keys kept on this computer, by service, while `remember_keys`.
    pub keys: BTreeMap<String, String>,
    pub remember_keys: bool,
}

impl Default for StudioPrefs {
    fn default() -> Self {
        StudioPrefs {
            image_provider: String::new(),
            models: BTreeMap::new(),
            op_providers: BTreeMap::new(),
            variants: BTreeMap::new(),
            aspect: "1:1".into(),
            resolution: "1K".into(),
            keys: BTreeMap::new(),
            remember_keys: true,
        }
    }
}

/// The text presets of Tools > Options > AI that are also catalog
/// services, so a key typed for one is kept for the other.
pub fn provider_of_preset(preset_key: &str) -> Option<&'static str> {
    match preset_key {
        "aipreset.anthropic" => Some("anthropic"),
        "aipreset.openai" => Some("openai"),
        "aipreset.gemini" => Some("gemini"),
        "aipreset.maritaca" => Some("maritaca"),
        "aipreset.deepseek" => Some("deepseek"),
        _ => None,
    }
}

/// `png` made to fit a service's limits: at most `max_pixels` in all and
/// `max_side` on its longer side (scaled down evenly, never up), and
/// at least `min_side` on its shorter one (scaled up when smaller). The
/// answer is a PNG and its size; the picture itself when it already fits.
pub fn fit_image(png: &[u8], max_pixels: u64, max_side: u32, min_side: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::load_from_memory(png)
        .map_err(|_| crate::i18n::tr("aiimg.unreadable_image"))?
        .to_rgba8();
    let (w, h) = (img.width().max(1), img.height().max(1));
    let mut k = 1.0f64;
    let px = w as f64 * h as f64;
    if max_pixels > 0 && px > max_pixels as f64 {
        k = (max_pixels as f64 / px).sqrt();
    }
    if max_side > 0 && (w.max(h) as f64 * k) > max_side as f64 {
        k = max_side as f64 / w.max(h) as f64;
    }
    if min_side > 0 && (w.min(h) as f64 * k) < min_side as f64 {
        k = min_side as f64 / w.min(h) as f64;
    }
    if (k - 1.0).abs() < 1e-9 {
        return Ok((png.to_vec(), w, h));
    }
    let nw = ((w as f64 * k).floor() as u32).max(1);
    let nh = ((h as f64 * k).floor() as u32).max(1);
    let out = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Lanczos3);
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(out)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok((bytes, nw, nh))
}

/// The size of a picture file in pixels.
pub fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    image::load_from_memory(bytes)
        .ok()
        .map(|i| (i.width(), i.height()))
}

#[cfg(test)]
pub(crate) fn test_png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(w, h, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]));
    let mut bytes = Vec::new();
    let _ = image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_for_an_english_only_service_are_translated_first() {
        use transport::{reply, Fake};
        let fake = Fake::new(
            |r| {
                if r.url.contains("anthropic") {
                    reply(200, serde_json::json!({"content": [{"type": "text", "text": "a red fox in the snow"}], "stop_reason": "end_turn"}))
                } else {
                    reply(200, serde_json::json!({"image": transport::base64(&test_png(4, 4)), "finish_reason": "SUCCESS"}))
                }
            },
            |_| Err("no".into()),
        );
        let req = OpRequest {
            op: Some(Op::Generate),
            provider: "stability".into(),
            model: "stable-image-core".into(),
            prompt: "uma raposa vermelha na neve".into(),
            aspect: "1:1".into(),
            english: true,
            translator: Some((crate::ai::AiPrefs::default(), "ak".into())),
            ..Default::default()
        };
        assert!(matches!(run(&fake, "sk", &req), Ok(Output::Images(_))));
        let sent = fake.sent();
        assert!(sent[0].body.contains("uma raposa"), "translation asked first");
        assert!(String::from_utf8_lossy(&sent[1].bytes).contains("a red fox in the snow"));
        // Without a service to translate: said so, nothing sent.
        let fake2 = Fake::new(|_| reply(500, serde_json::json!({})), |_| Err("no".into()));
        let no = OpRequest { translator: None, ..req.clone() };
        assert_eq!(run(&fake2, "sk", &no).expect_err("refused"), crate::i18n::tr("studio.english_only"));
        assert!(fake2.sent().is_empty());
        // English goes as it is.
        let en = OpRequest { prompt: "a red fox".into(), translator: None, ..req };
        assert!(run(&fake, "sk", &en).is_ok());
    }

    #[test]
    fn pictures_are_fitted_to_a_services_limits() {
        let png = test_png(2000, 1000);
        let (_, w, h) = fit_image(&png, 1_048_576, 0, 0).expect("fits");
        assert!(w as u64 * h as u64 <= 1_048_576 && (w as f64 / h as f64 - 2.0).abs() < 0.01);
        let (_, w, _) = fit_image(&png, 0, 1536, 0).expect("fits");
        assert_eq!(w, 1536);
        let (same, w, h) = fit_image(&png, 0, 0, 0).expect("fits");
        assert_eq!((same.len(), w, h), (png.len(), 2000, 1000));
        let small = test_png(40, 20);
        let (_, w, h) = fit_image(&small, 0, 0, 64).expect("fits");
        assert_eq!((w, h), (128, 64));
    }
}
