//! AI images: pictures made, remixed or cut out by an image service the
//! user sets up with their own key (Tools > Options > AI). TraceDraw has
//! no AI of its own; nothing is sent until the user asks.
//!
//! Two kinds of service are spoken to: Google's Gemini API
//! (`generateContent` with an image model) and the OpenAI Images API
//! (`images/generations` and `images/edits`), which other services and
//! local servers also offer at an address of the user's. Replies carry
//! the pictures as base64; they become bitmap objects on the page. See
//! docs/behavior/ai-generate.md.

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::net::{HttpRequest, HttpResponse, HttpResult};
use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::{Command, Rect, ShapeId};

/// The kinds of image service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageProvider {
    /// Google's Gemini API.
    #[default]
    Gemini,
    /// The OpenAI Images API (also offered by other services).
    OpenAi,
}

impl ImageProvider {
    pub const ALL: [ImageProvider; 2] = [ImageProvider::Gemini, ImageProvider::OpenAi];

    pub fn key(self) -> &'static str {
        match self {
            ImageProvider::Gemini => "aiimg.provider_gemini",
            ImageProvider::OpenAi => "aiimg.provider_openai",
        }
    }

    /// The service's base address when none is set.
    pub fn default_endpoint(self) -> &'static str {
        match self {
            ImageProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            ImageProvider::OpenAi => "https://api.openai.com/v1",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            ImageProvider::Gemini => "gemini-2.5-flash-image",
            ImageProvider::OpenAi => "gpt-image-1",
        }
    }

    /// Models offered before the service's own list is fetched.
    pub fn preset_models(self) -> &'static [&'static str] {
        match self {
            ImageProvider::Gemini => &["gemini-2.5-flash-image", "gemini-3-pro-image-preview"],
            ImageProvider::OpenAi => &["gpt-image-1", "gpt-image-1-mini", "dall-e-3"],
        }
    }
}

/// Ready-made image services: the i18n key of the name, the kind of
/// service and its address (empty: the usual one). The address stays
/// editable after one is picked.
pub const PRESETS: [(&str, ImageProvider, &str); 3] = [
    ("aipreset.gemini", ImageProvider::Gemini, ""),
    ("aipreset.openai", ImageProvider::OpenAi, ""),
    (
        "aipreset.localai",
        ImageProvider::OpenAi,
        "http://localhost:8080/v1",
    ),
];

/// The preset the image service matches, if any.
pub fn preset_of(prefs: &ImagePrefs) -> Option<usize> {
    PRESETS.iter().position(|(_, p, e)| {
        *p == prefs.provider && prefs.endpoint.trim().trim_end_matches('/') == *e
    })
}

/// The image service, kept with the preferences. The key is kept here
/// only while `remember_key` is on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ImagePrefs {
    pub provider: ImageProvider,
    /// Empty: the provider's usual address.
    pub endpoint: String,
    /// Empty: the provider's usual model.
    pub model: String,
    pub key: String,
    pub remember_key: bool,
}

impl ImagePrefs {
    pub fn endpoint(&self) -> String {
        let e = self.endpoint.trim().trim_end_matches('/');
        if e.is_empty() {
            self.provider.default_endpoint().to_string()
        } else {
            e.to_string()
        }
    }

    pub fn model(&self) -> String {
        let m = self.model.trim();
        if m.is_empty() {
            self.provider.default_model().to_string()
        } else {
            m.to_string()
        }
    }

    /// A service at an address of the user's may take no key.
    pub fn needs_key(&self) -> bool {
        self.endpoint.trim().is_empty()
    }
}

/// The key a session starts with: the remembered one, if any.
pub fn remembered_key(prefs: &ImagePrefs) -> String {
    if prefs.remember_key {
        prefs.key.clone()
    } else {
        String::new()
    }
}

/// The picture's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Aspect {
    #[default]
    Square,
    Landscape43,
    Landscape32,
    Wide169,
    Portrait34,
    Portrait23,
    Tall916,
}

impl Aspect {
    pub const ALL: [Aspect; 7] = [
        Aspect::Square,
        Aspect::Landscape43,
        Aspect::Landscape32,
        Aspect::Wide169,
        Aspect::Portrait34,
        Aspect::Portrait23,
        Aspect::Tall916,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Aspect::Square => "aiimg.aspect_square",
            Aspect::Landscape43 => "aiimg.aspect_43",
            Aspect::Landscape32 => "aiimg.aspect_32",
            Aspect::Wide169 => "aiimg.aspect_169",
            Aspect::Portrait34 => "aiimg.aspect_34",
            Aspect::Portrait23 => "aiimg.aspect_23",
            Aspect::Tall916 => "aiimg.aspect_916",
        }
    }

    /// The ratio as the Gemini API names it.
    pub fn ratio(self) -> &'static str {
        match self {
            Aspect::Square => "1:1",
            Aspect::Landscape43 => "4:3",
            Aspect::Landscape32 => "3:2",
            Aspect::Wide169 => "16:9",
            Aspect::Portrait34 => "3:4",
            Aspect::Portrait23 => "2:3",
            Aspect::Tall916 => "9:16",
        }
    }

    fn landscape(self) -> Option<bool> {
        match self {
            Aspect::Square => None,
            Aspect::Landscape43 | Aspect::Landscape32 | Aspect::Wide169 => Some(true),
            _ => Some(false),
        }
    }

    /// The nearest size the OpenAI Images API offers for `model`.
    pub fn openai_size(self, model: &str) -> &'static str {
        let dalle3 = model.starts_with("dall-e-3");
        if model.starts_with("dall-e-2") {
            return "1024x1024";
        }
        match (self.landscape(), dalle3) {
            (None, _) => "1024x1024",
            (Some(true), false) => "1536x1024",
            (Some(false), false) => "1024x1536",
            (Some(true), true) => "1792x1024",
            (Some(false), true) => "1024x1792",
        }
    }
}

/// Style presets: the i18n key of the name and the words added to the
/// prompt.
pub const STYLES: [(&str, &str); 6] = [
    (
        "aiimg.style_photo",
        "Photorealistic, natural light, sharp detail.",
    ),
    (
        "aiimg.style_line_art",
        "Black line art on white, clean even lines, no shading.",
    ),
    (
        "aiimg.style_watercolor",
        "Watercolour painting on textured paper, soft washes.",
    ),
    (
        "aiimg.style_illustration",
        "Flat editorial illustration, bold shapes.",
    ),
    (
        "aiimg.style_3d",
        "3D render, soft studio lighting, smooth materials.",
    ),
    (
        "aiimg.style_pop_art",
        "Pop art, bold outlines, halftone dots, saturated colours.",
    ),
];

/// Colour palette presets: the i18n key of the name, the words added to
/// the prompt and the colours of its swatch.
pub const PALETTES: [(&str, &str, [u32; 3]); 8] = [
    (
        "aiimg.palette_full",
        "Use the full colour spectrum.",
        [0xE53935, 0x43A047, 0x1E88E5],
    ),
    (
        "aiimg.palette_gray",
        "Only shades of grey.",
        [0x111111, 0x888888, 0xEEEEEE],
    ),
    (
        "aiimg.palette_warm",
        "A warm palette of reds, oranges and yellows.",
        [0xC62828, 0xEF6C00, 0xFDD835],
    ),
    (
        "aiimg.palette_cool",
        "A cool palette of blues, teals and violets.",
        [0x1A237E, 0x00897B, 0x7E57C2],
    ),
    (
        "aiimg.palette_pastel",
        "Soft pastel colours.",
        [0xF8BBD0, 0xB3E5FC, 0xC8E6C9],
    ),
    (
        "aiimg.palette_neon",
        "Vivid neon colours on dark tones.",
        [0xFF00E5, 0x00E5FF, 0x76FF03],
    ),
    (
        "aiimg.palette_earth",
        "Earthy browns, olives and sand.",
        [0x5D4037, 0x827717, 0xD7CCC8],
    ),
    (
        "aiimg.palette_mono_blue",
        "Shades of a single blue.",
        [0x0D47A1, 0x42A5F5, 0xBBDEFB],
    ),
];

/// The words that make a picture fit for tracing into vectors.
const VECTOR_PROMPT: &str = "Flat vector illustration: clean simple shapes, a few solid colours, no gradients, no textures, no shadows, on a plain white background.";
/// Remove Background, for services that can answer with transparency.
const CUTOUT_PROMPT: &str = "Remove the background completely and keep only the main subject, unchanged, at the same size and position, on a transparent background.";
/// Remove Background, for services that cannot: a flat key colour is
/// asked for and keyed out here.
const CHROMA_PROMPT: &str = "Keep the main subject exactly as it is, at the same size and position, and replace everything else with a flat, even, pure green background (#00FF00) with no shadows or gradients.";

/// The longest prompt sent, in characters.
pub const MAX_PROMPT: usize = 4000;
/// Pictures asked for at a time.
pub const MAX_COUNT: u8 = 4;
/// Results kept in the docker.
const MAX_RESULTS: usize = 12;
/// Largest reply taken, in bytes (pictures come as base64).
const MAX_REPLY: u64 = 64 * 1024 * 1024;
/// The undo steps.
const LABEL_GENERATE: &str = "AI Generate";
const LABEL_CUTOUT: &str = "Remove Background";

/// The full prompt: the user's words, then the style and palette.
pub fn compose_prompt(
    prompt: &str,
    style: Option<usize>,
    palette: Option<usize>,
    vector: bool,
) -> String {
    let mut out = String::new();
    if vector {
        out.push_str(VECTOR_PROMPT);
        out.push(' ');
    }
    out.push_str(prompt.trim());
    if let Some((_, words)) = style.and_then(|i| STYLES.get(i)) {
        if !vector {
            out.push_str("\n\nStyle: ");
            out.push_str(words);
        }
    }
    if let Some((_, words, _)) = palette.and_then(|i| PALETTES.get(i)) {
        out.push_str("\n\nColours: ");
        out.push_str(words);
    }
    out.chars().take(MAX_PROMPT).collect()
}

fn headers(prefs: &ImagePrefs, key: &str, json: bool) -> Vec<(String, String)> {
    let key = key.trim();
    let mut h = Vec::new();
    if json {
        h.push(("content-type".to_string(), "application/json".to_string()));
    }
    if !key.is_empty() {
        match prefs.provider {
            ImageProvider::Gemini => h.push(("x-goog-api-key".to_string(), key.to_string())),
            ImageProvider::OpenAi => h.push(("authorization".to_string(), format!("Bearer {key}"))),
        }
    }
    h
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// The request for a new picture (Gemini makes one per request; the
/// OpenAI API `count`).
pub fn build_generate(
    prefs: &ImagePrefs,
    key: &str,
    prompt: &str,
    aspect: Aspect,
    count: u8,
) -> HttpRequest {
    match prefs.provider {
        ImageProvider::Gemini => gemini_request(prefs, key, prompt, None, Some(aspect)),
        ImageProvider::OpenAi => {
            let model = prefs.model();
            let mut body = json!({
                "model": model,
                "prompt": prompt,
                "n": count.clamp(1, MAX_COUNT),
                "size": aspect.openai_size(&model),
            });
            if model.starts_with("dall-e") {
                body["response_format"] = json!("b64_json");
            }
            HttpRequest {
                method: "POST",
                url: format!("{}/images/generations", prefs.endpoint()),
                headers: headers(prefs, key, true),
                body: body.to_string(),
                bytes: Vec::new(),
            }
        }
    }
}

/// The request for a changed picture: `png` with `prompt`; `cutout` asks
/// for a transparent background where the service can give one.
pub fn build_edit(
    prefs: &ImagePrefs,
    key: &str,
    prompt: &str,
    png: &[u8],
    cutout: bool,
) -> HttpRequest {
    match prefs.provider {
        ImageProvider::Gemini => gemini_request(prefs, key, prompt, Some(png), None),
        ImageProvider::OpenAi => {
            let boundary = "tracedraw-ai-image-boundary-7f3a9c";
            let mut fields = vec![
                ("model", prefs.model()),
                ("prompt", prompt.to_string()),
                ("n", "1".to_string()),
            ];
            if cutout {
                fields.push(("background", "transparent".to_string()));
                fields.push(("output_format", "png".to_string()));
            }
            let mut body = Vec::new();
            for (name, value) in fields {
                body.extend_from_slice(
                    format!(
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                    )
                    .as_bytes(),
                );
            }
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"image.png\"\r\nContent-Type: image/png\r\n\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(png);
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            let mut h = headers(prefs, key, false);
            h.push((
                "content-type".to_string(),
                format!("multipart/form-data; boundary={boundary}"),
            ));
            HttpRequest {
                method: "POST",
                url: format!("{}/images/edits", prefs.endpoint()),
                headers: h,
                body: String::new(),
                bytes: body,
            }
        }
    }
}

fn gemini_request(
    prefs: &ImagePrefs,
    key: &str,
    prompt: &str,
    png: Option<&[u8]>,
    aspect: Option<Aspect>,
) -> HttpRequest {
    let mut parts = Vec::new();
    if let Some(png) = png {
        parts.push(json!({"inline_data": {"mime_type": "image/png", "data": b64(png)}}));
    }
    parts.push(json!({"text": prompt}));
    let mut config = json!({"responseModalities": ["TEXT", "IMAGE"]});
    if let Some(a) = aspect {
        config["imageConfig"] = json!({"aspectRatio": a.ratio()});
    }
    let body = json!({
        "contents": [{"role": "user", "parts": parts}],
        "generationConfig": config,
    });
    HttpRequest {
        method: "POST",
        url: format!(
            "{}/models/{}:generateContent",
            prefs.endpoint(),
            prefs.model()
        ),
        headers: headers(prefs, key, true),
        body: body.to_string(),
        bytes: Vec::new(),
    }
}

/// The request for the service's models.
pub fn build_models(prefs: &ImagePrefs, key: &str) -> HttpRequest {
    HttpRequest {
        method: "GET",
        url: format!("{}/models", prefs.endpoint()),
        headers: headers(prefs, key, false),
        body: String::new(),
        bytes: Vec::new(),
    }
}

fn http_error(r: &HttpResponse, v: Option<&Value>) -> String {
    let message = v
        .and_then(|v| v.pointer("/error/message").and_then(|m| m.as_str()))
        .map(str::to_string)
        .unwrap_or_else(|| r.body.chars().take(200).collect());
    trf(
        "ai.http_error",
        &[
            ("status", &r.status.to_string()),
            ("message", message.trim()),
        ],
    )
}

/// The image models a service lists.
pub fn parse_models(provider: ImageProvider, r: &HttpResponse) -> Result<Vec<String>, String> {
    let v: Option<Value> = serde_json::from_str(&r.body).ok();
    if !(200..300).contains(&r.status) {
        return Err(http_error(r, v.as_ref()));
    }
    let v = v.ok_or_else(|| tr("ai.unreadable_reply"))?;
    let ids: Vec<String> = match provider {
        ImageProvider::Gemini => v
            .get("models")
            .and_then(|m| m.as_array())
            .map(|l| {
                l.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .map(|n| n.trim_start_matches("models/").to_string())
                    .filter(|n| n.contains("image"))
                    .collect()
            })
            .unwrap_or_default(),
        ImageProvider::OpenAi => v
            .get("data")
            .and_then(|d| d.as_array())
            .map(|l| {
                l.iter()
                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()))
                    .filter(|i| i.contains("image") || i.starts_with("dall-e"))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    };
    if ids.is_empty() {
        return Err(tr("ai.no_models"));
    }
    Ok(ids)
}

/// The pictures of a reply (as the bytes of their files), or why there
/// are none.
pub fn parse_images(provider: ImageProvider, r: &HttpResponse) -> Result<Vec<Vec<u8>>, String> {
    let v: Option<Value> = serde_json::from_str(&r.body).ok();
    if !(200..300).contains(&r.status) {
        return Err(http_error(r, v.as_ref()));
    }
    let v = v.ok_or_else(|| tr("ai.unreadable_reply"))?;
    let mut encoded: Vec<&str> = Vec::new();
    let mut said = String::new();
    match provider {
        ImageProvider::Gemini => {
            if let Some(reason) = v
                .pointer("/promptFeedback/blockReason")
                .and_then(|b| b.as_str())
            {
                return Err(trf("aiimg.refused", &[("why", reason)]));
            }
            for c in v
                .get("candidates")
                .and_then(|c| c.as_array())
                .into_iter()
                .flatten()
            {
                for p in c
                    .pointer("/content/parts")
                    .and_then(|p| p.as_array())
                    .into_iter()
                    .flatten()
                {
                    let data = p
                        .get("inlineData")
                        .or_else(|| p.get("inline_data"))
                        .and_then(|d| d.get("data"))
                        .and_then(|d| d.as_str());
                    if let Some(d) = data {
                        encoded.push(d);
                    } else if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                        said.push_str(t);
                    }
                }
            }
        }
        ImageProvider::OpenAi => {
            for d in v
                .get("data")
                .and_then(|d| d.as_array())
                .into_iter()
                .flatten()
            {
                if let Some(s) = d.get("b64_json").and_then(|s| s.as_str()) {
                    encoded.push(s);
                } else if d.get("url").is_some() {
                    return Err(tr("aiimg.url_reply"));
                }
            }
        }
    }
    let images: Vec<Vec<u8>> = encoded
        .into_iter()
        .filter_map(|s| {
            base64::engine::general_purpose::STANDARD
                .decode(s.trim())
                .ok()
        })
        .collect();
    if images.is_empty() {
        let said = said.trim();
        return Err(if said.is_empty() {
            tr("aiimg.no_image")
        } else {
            trf(
                "aiimg.no_image_said",
                &[("text", &said.chars().take(300).collect::<String>())],
            )
        });
    }
    Ok(images)
}

/// A picture file (PNG, JPEG or WebP) as PNG with its size.
pub fn to_png(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    Some((crate::bitmap_fx::encode(&img)?, w, h))
}

/// Make the pure green a service painted behind the subject transparent:
/// full green goes, colours far from it stay, and edges in between keep
/// part of their opacity with the green taken out of them.
pub fn chroma_key(img: &mut image::RgbaImage) {
    const LOW: f32 = 40.0;
    const HIGH: f32 = 120.0;
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        let others = r.max(b) as f32;
        let green = g as f32 - others;
        if green <= LOW {
            continue;
        }
        let keep = ((HIGH - green) / (HIGH - LOW)).clamp(0.0, 1.0);
        // Take the green spill out of what stays.
        let g2 = (g as f32).min(others).round() as u8;
        p.0 = [r, g2, b, (a as f32 * keep).round() as u8];
    }
}

/// The picture already has see-through parts.
fn has_transparency(img: &image::RgbaImage) -> bool {
    img.pixels().any(|p| p.0[3] < 250)
}

/// A picture the service made.
#[derive(Debug, Clone, PartialEq)]
pub struct GenImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// What the docker makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Create,
    Remix,
    Vector,
}

/// Where the requests stand.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ImageStatus {
    #[default]
    Idle,
    /// `done` of `total` answers are in.
    Waiting {
        done: usize,
        total: usize,
    },
    /// This many pictures were made.
    Done(usize),
    Listed(usize),
    Failed(String),
}

enum Job {
    Generate { vector: bool },
    Remix { source: ShapeId },
    Cutout { shape: ShapeId, chroma: bool },
    Models,
}

struct Pending {
    rx: std::sync::mpsc::Receiver<HttpResult>,
    job: Job,
}

/// The image service's state for the session.
pub struct ImageSession {
    pub mode: Mode,
    pub prompt: String,
    pub remix_prompt: String,
    pub aspect: Aspect,
    pub style: Option<usize>,
    pub palette: Option<usize>,
    pub count: u8,
    pub key: String,
    pub status: ImageStatus,
    /// Newest first.
    pub results: Vec<GenImage>,
    pub models: Vec<String>,
    pending: Vec<Pending>,
    total: usize,
    made: usize,
    errors: Vec<String>,
    /// Tests: answers requests at once instead of the network.
    pub fake: Option<fn(&HttpRequest) -> HttpResult>,
    /// Tests: the requests sent, newest last.
    pub sent: Vec<HttpRequest>,
}

impl Default for ImageSession {
    fn default() -> Self {
        ImageSession {
            mode: Mode::default(),
            prompt: String::new(),
            remix_prompt: String::new(),
            aspect: Aspect::default(),
            style: None,
            palette: None,
            count: 1,
            key: String::new(),
            status: ImageStatus::default(),
            results: Vec::new(),
            models: Vec::new(),
            pending: Vec::new(),
            total: 0,
            made: 0,
            errors: Vec::new(),
            fake: None,
            sent: Vec::new(),
        }
    }
}

impl ImageSession {
    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }
}

impl App {
    /// AI features are on (Tools > Options > AI).
    pub fn ai_features_on(&self) -> bool {
        self.settings.ai_enabled
    }

    /// The image service is set up and AI features are on.
    pub fn ai_image_ready(&self) -> bool {
        let p = &self.settings.ai_image;
        self.ai_features_on()
            && !p.model().is_empty()
            && (!p.needs_key() || !self.ai_image.key.trim().is_empty())
    }

    /// The drawing service (AI Drawing) is set up and AI features are on.
    pub fn ai_drawing_ready(&self) -> bool {
        self.ai_features_on() && self.ai_ready()
    }

    /// The one bitmap selected, for Remix and Remove Background.
    pub fn ai_source_bitmap(&self) -> Option<(ShapeId, Vec<u8>)> {
        let sel = self.selected_shapes();
        let [s] = sel.as_slice() else {
            return None;
        };
        match &s.kind {
            ShapeKind::Bitmap { png, .. } => Some((s.id, png.clone())),
            _ => None,
        }
    }

    fn ai_image_problem(&self) -> Option<String> {
        if !self.ai_features_on() {
            Some(tr("aiimg.off"))
        } else if !self.ai_image_ready() {
            Some(tr("aiimg.not_set_up"))
        } else if !self.has_document() {
            Some(tr("ai.need_document"))
        } else {
            None
        }
    }

    fn ai_image_send(&mut self, http: HttpRequest, job: Job) {
        let (tx, rx) = std::sync::mpsc::channel();
        match self.ai_image.fake {
            Some(fake) => {
                let _ = tx.send(fake(&http));
            }
            None => crate::net::spawn(http.clone(), MAX_REPLY, tx),
        }
        self.ai_image.sent.push(http);
        if self.ai_image.sent.len() > 8 {
            self.ai_image.sent.remove(0);
        }
        self.ai_image.pending.push(Pending { rx, job });
    }

    fn ai_image_begin(&mut self, total: usize) {
        self.ai_image.total = total;
        self.ai_image.made = 0;
        self.ai_image.errors.clear();
        self.ai_image.status = ImageStatus::Waiting { done: 0, total };
    }

    /// The docker's Generate, for its mode.
    pub fn ai_generate(&mut self) {
        match self.ai_image.mode {
            Mode::Create => self.ai_create(false),
            Mode::Vector => self.ai_create(true),
            Mode::Remix => self.ai_remix(),
        }
    }

    /// Create image (or vectors): pictures from the prompt, placed on the
    /// page.
    pub fn ai_create(&mut self, vector: bool) {
        if self.ai_image.busy() {
            return;
        }
        if let Some(p) = self.ai_image_problem() {
            self.ai_image.status = ImageStatus::Failed(p);
            return;
        }
        let words = self.ai_image.prompt.trim().to_string();
        if words.is_empty() {
            self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_prompt"));
            return;
        }
        let s = &self.ai_image;
        let prompt = compose_prompt(&words, s.style, s.palette, vector);
        let count = s.count.clamp(1, MAX_COUNT);
        let prefs = self.settings.ai_image.clone();
        let key = self.ai_image.key.clone();
        let aspect = self.ai_image.aspect;
        // Gemini answers one picture a request.
        let requests = match prefs.provider {
            ImageProvider::Gemini => count as usize,
            ImageProvider::OpenAi => 1,
        };
        self.ai_image_begin(requests);
        for _ in 0..requests {
            let http = build_generate(&prefs, &key, &prompt, aspect, count);
            self.ai_image_send(http, Job::Generate { vector });
        }
        self.poll_ai_image();
    }

    /// Remix image: the selected bitmap changed as the prompt says.
    pub fn ai_remix(&mut self) {
        if self.ai_image.busy() {
            return;
        }
        if let Some(p) = self.ai_image_problem() {
            self.ai_image.status = ImageStatus::Failed(p);
            return;
        }
        let Some((source, png)) = self.ai_source_bitmap() else {
            self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_bitmap"));
            return;
        };
        let words = self.ai_image.remix_prompt.trim().to_string();
        if words.is_empty() {
            self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_prompt"));
            return;
        }
        let prompt = compose_prompt(&words, self.ai_image.style, self.ai_image.palette, false);
        let http = build_edit(
            &self.settings.ai_image,
            &self.ai_image.key,
            &prompt,
            &png,
            false,
        );
        self.ai_image_begin(1);
        self.ai_image_send(http, Job::Remix { source });
        self.poll_ai_image();
    }

    /// Bitmaps > Remove Background: the selected bitmap's subject kept,
    /// the rest made transparent, as one undo step.
    pub fn ai_remove_background(&mut self) {
        if self.ai_image.busy() {
            return;
        }
        if let Some(p) = self.ai_image_problem() {
            self.status = p.clone();
            self.ai_image.status = ImageStatus::Failed(p);
            return;
        }
        let Some((shape, png)) = self.ai_source_bitmap() else {
            self.status = tr("aiimg.need_bitmap");
            return;
        };
        let prefs = self.settings.ai_image.clone();
        let chroma = prefs.provider == ImageProvider::Gemini;
        let prompt = if chroma { CHROMA_PROMPT } else { CUTOUT_PROMPT };
        let http = build_edit(&prefs, &self.ai_image.key, prompt, &png, !chroma);
        self.ai_image_begin(1);
        self.status = tr("aiimg.removing_background");
        self.ai_image_send(http, Job::Cutout { shape, chroma });
        self.poll_ai_image();
    }

    /// Fetch the service's image models.
    pub fn ai_image_fetch_models(&mut self) {
        if self.ai_image.busy() {
            return;
        }
        let prefs = self.settings.ai_image.clone();
        if prefs.needs_key() && self.ai_image.key.trim().is_empty() {
            self.ai_image.status = ImageStatus::Failed(tr("ai.need_key"));
            return;
        }
        let http = build_models(&prefs, &self.ai_image.key);
        self.ai_image_begin(1);
        self.ai_image_send(http, Job::Models);
        self.poll_ai_image();
    }

    /// Stop waiting (answers are dropped when they come).
    pub fn ai_image_cancel(&mut self) {
        if !self.ai_image.pending.is_empty() {
            self.ai_image.pending.clear();
            self.ai_image.status = ImageStatus::Idle;
        }
    }

    /// Take in answers that arrived. True while some are still on their
    /// way.
    pub fn poll_ai_image(&mut self) -> bool {
        let mut i = 0;
        while i < self.ai_image.pending.len() {
            let got = match self.ai_image.pending[i].rx.try_recv() {
                Ok(r) => Some(r),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err(tr("ai.connection_lost")))
                }
            };
            match got {
                Some(result) => {
                    let p = self.ai_image.pending.remove(i);
                    self.ai_image_receive(result, p.job);
                }
                None => i += 1,
            }
        }
        self.ai_image.busy()
    }

    fn ai_image_receive(&mut self, result: HttpResult, job: Job) {
        let provider = self.settings.ai_image.provider;
        let done = self.ai_image.total - self.ai_image.pending.len();
        if let Job::Models = job {
            match result.and_then(|r| parse_models(provider, &r)) {
                Ok(m) => {
                    self.ai_image.status = ImageStatus::Listed(m.len());
                    self.ai_image.models = m;
                }
                Err(e) => self.ai_image.status = ImageStatus::Failed(e),
            }
            return;
        }
        let images = result
            .and_then(|r| parse_images(provider, &r))
            .and_then(|files| {
                let pngs: Vec<GenImage> = files
                    .iter()
                    .filter_map(|f| to_png(f))
                    .map(|(png, width, height)| GenImage { png, width, height })
                    .collect();
                if pngs.is_empty() {
                    Err(tr("aiimg.unreadable_image"))
                } else {
                    Ok(pngs)
                }
            });
        match images {
            Err(e) => self.ai_image.errors.push(e),
            Ok(images) => match job {
                Job::Generate { vector } => {
                    for img in images {
                        self.ai_place_image(&img, None, vector);
                        self.ai_keep_result(img);
                    }
                }
                Job::Remix { source } => {
                    if let Some(img) = images.into_iter().next() {
                        self.ai_place_image(&img, Some(source), false);
                        self.ai_keep_result(img);
                    }
                }
                Job::Cutout { shape, chroma } => {
                    if let Some(img) = images.into_iter().next() {
                        self.ai_apply_cutout(shape, img, chroma);
                    }
                }
                Job::Models => {}
            },
        }
        self.ai_image.status = if self.ai_image.busy() {
            ImageStatus::Waiting {
                done,
                total: self.ai_image.total,
            }
        } else if self.ai_image.made == 0 {
            ImageStatus::Failed(
                self.ai_image
                    .errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| tr("aiimg.no_image")),
            )
        } else {
            ImageStatus::Done(self.ai_image.made)
        };
    }

    fn ai_keep_result(&mut self, img: GenImage) {
        self.ai_image.results.insert(0, img);
        self.ai_image.results.truncate(MAX_RESULTS);
    }

    /// Put a picture on the page: over `over` (a remix, same place and
    /// size) or centred on the page within four fifths of it. `vector`
    /// traces it into shapes.
    pub fn ai_place_image(&mut self, img: &GenImage, over: Option<ShapeId>, vector: bool) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let (rect, transform) = match over.and_then(|id| self.doc().find_shape(id).cloned()) {
            Some(src) => match &src.kind {
                ShapeKind::Bitmap { rect, .. } => (*rect, src.transform),
                _ => (Rect::ZERO, tracedraw_core::Affine::IDENTITY),
            },
            None => (Rect::ZERO, tracedraw_core::Affine::IDENTITY),
        };
        let rect = if rect.area() > 0.0 {
            // A remix keeps the source's height and its own proportions.
            let w = rect.height() * img.width as f64 / img.height.max(1) as f64;
            Rect::new(rect.x0, rect.y0, rect.x0 + w, rect.y1)
        } else {
            let page = self.page_size();
            let scale =
                (page.width * 0.8 / img.width as f64).min(page.height * 0.8 / img.height as f64);
            let (w, h) = (img.width as f64 * scale, img.height as f64 * scale);
            let (x, y) = ((page.width - w) / 2.0, (page.height - h) / 2.0);
            Rect::new(x, y, x + w, y + h)
        };
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(
            id,
            ShapeKind::Bitmap {
                rect,
                width_px: img.width,
                height_px: img.height,
                png: img.png.clone(),
                fx: None,
            },
        );
        shape.transform = transform;
        shape.fill = tracedraw_core::Fill::None;
        shape.stroke = None;
        shape.name = Some(tr("aiimg.object_name"));
        if let Err(e) = self
            .engine
            .run_batch(LABEL_GENERATE, &[Command::AddShape { layer, shape }])
        {
            self.ai_image.errors.push(e.to_string());
            return;
        }
        self.ai_image.made += 1;
        self.select(vec![id]);
        if vector {
            let mut settings = self
                .settings
                .tracing
                .settings(crate::trace::Preset::Clipart);
            settings.delete_original = true;
            self.trace_bitmaps(&settings);
        }
        self.status = trf(
            "status.ai_image_placed",
            &[("n", &self.ai_image.made.to_string())],
        );
    }

    fn ai_apply_cutout(&mut self, shape: ShapeId, img: GenImage, chroma: bool) {
        let Some(src) = self.doc().find_shape(shape).cloned() else {
            self.ai_image.errors.push(tr("aiimg.source_gone"));
            return;
        };
        let ShapeKind::Bitmap { rect, .. } = &src.kind else {
            self.ai_image.errors.push(tr("aiimg.source_gone"));
            return;
        };
        let Some(mut out) = crate::bitmap_fx::decode(&img.png) else {
            self.ai_image.errors.push(tr("aiimg.unreadable_image"));
            return;
        };
        if chroma || !has_transparency(&out) {
            chroma_key(&mut out);
        }
        let Some(png) = crate::bitmap_fx::encode(&out) else {
            return;
        };
        let cmd = Command::SetShapeKind {
            shape,
            kind: ShapeKind::Bitmap {
                rect: *rect,
                width_px: out.width(),
                height_px: out.height(),
                png,
                fx: None,
            },
        };
        match self.engine.run_batch(LABEL_CUTOUT, &[cmd]) {
            Ok(_) => {
                self.ai_image.made += 1;
                self.status = tr("status.background_removed");
            }
            Err(e) => self.ai_image.errors.push(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let img = RgbaImage::from_fn(w, h, |x, y| Rgba(f(x, y)));
        crate::bitmap_fx::encode(&img).expect("png")
    }

    fn gemini_reply(png: &[u8]) -> HttpResult {
        Ok(HttpResponse {
            status: 200,
            body: json!({"candidates": [{"content": {"parts": [
                {"text": "Here it is"},
                {"inlineData": {"mimeType": "image/png", "data": b64(png)}}
            ]}}]})
            .to_string(),
        })
    }

    fn ready_app(provider: ImageProvider) -> App {
        let mut app = App::headless();
        app.settings.ai_enabled = true;
        app.settings.ai_image.provider = provider;
        app.ai_image.key = "k".into();
        app
    }

    #[test]
    fn requests_are_built_for_both_kinds_of_service() {
        let mut p = ImagePrefs::default();
        let r = build_generate(&p, "key1", "a cat", Aspect::Wide169, 2);
        assert_eq!(
            r.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent"
        );
        assert!(r
            .headers
            .contains(&("x-goog-api-key".into(), "key1".into())));
        let v: Value = serde_json::from_str(&r.body).expect("json");
        assert_eq!(
            v.pointer("/contents/0/parts/0/text")
                .and_then(|t| t.as_str()),
            Some("a cat")
        );
        assert_eq!(
            v.pointer("/generationConfig/imageConfig/aspectRatio")
                .and_then(|t| t.as_str()),
            Some("16:9")
        );
        let e = build_edit(&p, "key1", "make it blue", b"PNG", false);
        let v: Value = serde_json::from_str(&e.body).expect("json");
        assert_eq!(
            v.pointer("/contents/0/parts/0/inline_data/data")
                .and_then(|t| t.as_str()),
            Some(b64(b"PNG").as_str())
        );
        p.provider = ImageProvider::OpenAi;
        let r = build_generate(&p, "key2", "a dog", Aspect::Portrait23, 9);
        assert_eq!(r.url, "https://api.openai.com/v1/images/generations");
        assert!(r
            .headers
            .contains(&("authorization".into(), "Bearer key2".into())));
        let v: Value = serde_json::from_str(&r.body).expect("json");
        assert_eq!(v["n"], 4, "at most four at a time");
        assert_eq!(v["size"], "1024x1536");
        assert!(v.get("response_format").is_none());
        p.model = "dall-e-3".into();
        let v: Value = serde_json::from_str(&build_generate(&p, "", "x", Aspect::Wide169, 1).body)
            .expect("json");
        assert_eq!(
            (v["size"].as_str(), v["response_format"].as_str()),
            (Some("1792x1024"), Some("b64_json"))
        );
        p.model.clear();
        let e = build_edit(&p, "key2", "cut", b"\x89PNGDATA", true);
        assert_eq!(e.url, "https://api.openai.com/v1/images/edits");
        assert!(e.body.is_empty() && !e.bytes.is_empty());
        let text = String::from_utf8_lossy(&e.bytes);
        assert!(text.contains("name=\"background\"\r\n\r\ntransparent"));
        assert!(text.contains("filename=\"image.png\"") && text.contains("PNGDATA"));
        assert!(e
            .headers
            .iter()
            .any(|(k, v)| k == "content-type" && v.starts_with("multipart/form-data; boundary=")));
        // A service at the user's own address may take no key.
        p.endpoint = "http://localhost:8080/v1/".into();
        assert!(!p.needs_key());
        let r = build_generate(&p, "", "x", Aspect::Square, 1);
        assert_eq!(r.url, "http://localhost:8080/v1/images/generations");
        assert!(!r.headers.iter().any(|(k, _)| k == "authorization"));
    }

    #[test]
    fn prompts_carry_style_palette_and_vector_words() {
        let p = compose_prompt("  a fox ", Some(2), Some(1), false);
        assert!(p.starts_with("a fox") && p.contains("Watercolour") && p.contains("grey"));
        let v = compose_prompt("a fox", Some(0), None, true);
        assert!(v.starts_with("Flat vector") && !v.contains("Photorealistic"));
        assert_eq!(
            compose_prompt(&"x".repeat(9000), None, None, false)
                .chars()
                .count(),
            MAX_PROMPT
        );
    }

    #[test]
    fn replies_and_errors_are_read() {
        let one = png(2, 2, |_, _| [1, 2, 3, 255]);
        let r = gemini_reply(&one).expect("reply");
        assert_eq!(
            parse_images(ImageProvider::Gemini, &r).expect("images"),
            vec![one.clone()]
        );
        let r = HttpResponse {
            status: 200,
            body: json!({"data": [{"b64_json": b64(&one)}, {"b64_json": b64(&one)}]}).to_string(),
        };
        assert_eq!(
            parse_images(ImageProvider::OpenAi, &r)
                .expect("images")
                .len(),
            2
        );
        let bad = HttpResponse {
            status: 401,
            body: json!({"error": {"message": "Incorrect API key"}}).to_string(),
        };
        assert!(parse_images(ImageProvider::OpenAi, &bad)
            .unwrap_err()
            .contains("Incorrect API key"));
        let blocked = HttpResponse {
            status: 200,
            body: json!({"promptFeedback": {"blockReason": "SAFETY"}}).to_string(),
        };
        assert!(parse_images(ImageProvider::Gemini, &blocked)
            .unwrap_err()
            .contains("SAFETY"));
        let words = HttpResponse {
            status: 200,
            body: json!({"candidates": [{"content": {"parts": [{"text": "I cannot draw that"}]}}]})
                .to_string(),
        };
        assert!(parse_images(ImageProvider::Gemini, &words)
            .unwrap_err()
            .contains("I cannot draw that"));
        let url = HttpResponse {
            status: 200,
            body: json!({"data": [{"url": "https://x"}]}).to_string(),
        };
        assert!(parse_images(ImageProvider::OpenAi, &url).is_err());
        let junk = HttpResponse {
            status: 200,
            body: "not json".into(),
        };
        assert!(parse_images(ImageProvider::Gemini, &junk).is_err());
        assert!(to_png(b"not a picture").is_none());
        let models = HttpResponse {
            status: 200,
            body: json!({"models": [{"name": "models/gemini-2.5-flash"}, {"name": "models/gemini-2.5-flash-image"}]}).to_string(),
        };
        assert_eq!(
            parse_models(ImageProvider::Gemini, &models).expect("models"),
            vec!["gemini-2.5-flash-image"]
        );
    }

    #[test]
    fn green_is_keyed_out_and_edges_keep_part_of_their_opacity() {
        let mut img = RgbaImage::from_fn(4, 1, |x, _| {
            Rgba(match x {
                0 => [0, 255, 0, 255],
                1 => [200, 30, 40, 255],
                2 => [90, 170, 90, 255],
                _ => [250, 250, 250, 255],
            })
        });
        chroma_key(&mut img);
        assert_eq!(img.get_pixel(0, 0).0[3], 0);
        assert_eq!(img.get_pixel(1, 0).0, [200, 30, 40, 255]);
        let edge = img.get_pixel(2, 0).0;
        assert!(edge[3] > 0 && edge[3] < 255, "{edge:?}");
        assert!(edge[1] <= 90, "green spill taken out: {edge:?}");
        assert_eq!(img.get_pixel(3, 0).0[3], 255);
    }

    #[test]
    fn nothing_is_sent_while_ai_is_off_or_not_set_up() {
        let mut app = App::headless();
        app.ai_image.fake = Some(|_| panic!("nothing may be sent"));
        app.ai_image.prompt = "a cat".into();
        app.settings.ai_enabled = false;
        app.ai_image.key = "k".into();
        app.ai_create(false);
        assert_eq!(app.ai_image.status, ImageStatus::Failed(tr("aiimg.off")));
        app.settings.ai_enabled = true;
        app.ai_image.key.clear();
        assert!(!app.ai_image_ready());
        app.ai_create(false);
        assert_eq!(
            app.ai_image.status,
            ImageStatus::Failed(tr("aiimg.not_set_up"))
        );
        app.ai_image.key = "k".into();
        app.ai_image.prompt.clear();
        app.ai_create(false);
        assert_eq!(
            app.ai_image.status,
            ImageStatus::Failed(tr("aiimg.need_prompt"))
        );
        app.ai_remove_background();
        assert!(app.ai_image.sent.is_empty());
    }

    fn count_bitmaps(app: &App) -> usize {
        app.doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter())
            .filter(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
            .count()
    }

    #[test]
    fn created_pictures_go_on_the_page_one_undo_step_each() {
        let mut app = ready_app(ImageProvider::Gemini);
        app.ai_image.fake = Some(|_| gemini_reply(&png(40, 20, |_, _| [200, 10, 10, 255])));
        app.ai_image.prompt = "a red banner".into();
        app.ai_image.count = 2;
        app.ai_create(false);
        assert_eq!(app.ai_image.sent.len(), 2, "Gemini: one request a picture");
        assert_eq!(app.ai_image.status, ImageStatus::Done(2));
        assert_eq!(count_bitmaps(&app), 2);
        assert_eq!(app.ai_image.results.len(), 2);
        // Centred on the page within four fifths of its width.
        let s = app.selected_shapes().pop().expect("selected");
        let ShapeKind::Bitmap { rect, .. } = s.kind else {
            panic!()
        };
        let page = app.page_size();
        assert!((rect.width() - page.width * 0.8).abs() < 1e-6);
        assert!(((rect.x0 + rect.x1) / 2.0 - page.width / 2.0).abs() < 1e-6);
        app.undo();
        assert_eq!(count_bitmaps(&app), 1);
    }

    #[test]
    fn vectors_are_traced_from_the_picture() {
        let mut app = ready_app(ImageProvider::Gemini);
        app.ai_image.fake = Some(|_| {
            gemini_reply(&png(64, 64, |x, y| {
                if (x as i32 - 32).pow(2) + (y as i32 - 32).pow(2) < 400 {
                    [20, 60, 200, 255]
                } else {
                    [255, 255, 255, 255]
                }
            }))
        });
        app.ai_image.mode = Mode::Vector;
        app.ai_image.prompt = "a blue dot".into();
        app.ai_generate();
        assert_eq!(app.ai_image.status, ImageStatus::Done(1));
        assert_eq!(
            count_bitmaps(&app),
            0,
            "the picture is replaced by its trace"
        );
        let groups = app
            .doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter())
            .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .count();
        assert_eq!(groups, 1);
    }

    #[test]
    fn remix_and_remove_background_work_on_the_selected_bitmap() {
        let mut app = ready_app(ImageProvider::Gemini);
        let src = png(8, 8, |_, _| [255, 255, 255, 255]);
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: Rect::new(10.0, 10.0, 50.0, 50.0),
                width_px: 8,
                height_px: 8,
                png: src,
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        // Remove Background through a service that paints the back green.
        app.ai_image.fake = Some(|_| {
            gemini_reply(&png(8, 8, |x, _| {
                if x < 4 {
                    [0, 255, 0, 255]
                } else {
                    [180, 40, 40, 255]
                }
            }))
        });
        app.ai_remove_background();
        assert_eq!(app.ai_image.status, ImageStatus::Done(1));
        let body: Value = serde_json::from_str(&app.ai_image.sent[0].body).expect("json");
        assert!(body.to_string().contains("#00FF00"));
        let s = app.doc().find_shape(id).cloned().expect("shape");
        let ShapeKind::Bitmap { png: out, rect, .. } = &s.kind else {
            panic!()
        };
        assert_eq!(*rect, Rect::new(10.0, 10.0, 50.0, 50.0), "stays in place");
        let img = crate::bitmap_fx::decode(out).expect("png");
        assert_eq!(img.get_pixel(0, 0).0[3], 0);
        assert_eq!(img.get_pixel(7, 0).0[3], 255);
        app.undo();
        let s = app.doc().find_shape(id).cloned().expect("shape");
        let ShapeKind::Bitmap { png: back, .. } = &s.kind else {
            panic!()
        };
        assert_eq!(
            crate::bitmap_fx::decode(back)
                .expect("png")
                .get_pixel(0, 0)
                .0,
            [255, 255, 255, 255]
        );
        // Remix adds the changed picture over the source.
        app.select(vec![id]);
        app.ai_image.mode = Mode::Remix;
        app.ai_image.remix_prompt = "make it red".into();
        app.ai_image.fake = Some(|_| gemini_reply(&png(16, 8, |_, _| [255, 0, 0, 255])));
        app.ai_generate();
        assert_eq!(count_bitmaps(&app), 2);
        let s = app.selected_shapes().pop().expect("remix");
        let ShapeKind::Bitmap { rect, .. } = s.kind else {
            panic!()
        };
        assert!((rect.height() - 40.0).abs() < 1e-9 && (rect.width() - 80.0).abs() < 1e-9);
    }

    #[test]
    fn errors_of_the_service_are_shown_and_nothing_is_placed() {
        let mut app = ready_app(ImageProvider::OpenAi);
        app.ai_image.fake = Some(|_| {
            Ok(HttpResponse {
                status: 429,
                body: json!({"error": {"message": "Rate limit reached"}}).to_string(),
            })
        });
        app.ai_image.prompt = "x".into();
        app.ai_create(false);
        match &app.ai_image.status {
            ImageStatus::Failed(e) => assert!(e.contains("429") && e.contains("Rate limit")),
            s => panic!("{s:?}"),
        }
        assert_eq!(count_bitmaps(&app), 0);
        app.ai_image.fake = Some(|_| Err("network down".into()));
        app.ai_create(false);
        assert_eq!(
            app.ai_image.status,
            ImageStatus::Failed("network down".into())
        );
    }

    #[test]
    fn the_key_is_kept_only_when_asked() {
        let mut prefs = ImagePrefs {
            key: "secret".into(),
            ..Default::default()
        };
        assert_eq!(remembered_key(&prefs), "");
        prefs.remember_key = true;
        assert_eq!(remembered_key(&prefs), "secret");
        let app = App::headless();
        let json = serde_json::to_string(&app.settings).expect("settings");
        assert!(!json.contains("secret"));
    }
}
