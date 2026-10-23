//! The studio inside the editor: keys by service, requests run on worker
//! threads, and what their pictures do to the drawing.

use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;

use crate::ai_image::{GenImage, ImageStatus};
use crate::app::App;
use crate::i18n::{tr, trf};
use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::{Command, Rect, ShapeId};

use super::transport::{Net, Transport};
use super::{Catalog, Op, OpRequest, Output};

/// Undo labels.
const LABEL_STUDIO: &str = "AI Studio";

/// Where a result goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// A new picture on the page.
    New,
    /// Over a picture it was made from (remix), same place and height.
    Over(ShapeId),
    /// In place of the picture's pixels, same frame (upscale, relight,
    /// new background).
    Replace(ShapeId),
    /// The picture's background made transparent.
    Cutout(ShapeId),
    /// The picture grown by these many of its pixels on each side.
    Expand {
        shape: ShapeId,
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    },
    /// Curves in place of the picture.
    Vector(ShapeId),
}

struct Job {
    rx: Receiver<Result<Output, String>>,
    target: Target,
}

/// Where a key test stands.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyTest {
    Testing,
    Works(String),
    Fails(String),
}

/// The studio's state for the session.
pub struct Studio {
    pub catalog: Catalog,
    /// Keys in use, by service (also in the preferences when remembered).
    pub keys: BTreeMap<String, String>,
    /// The operation the Edit page shows.
    pub op: Op,
    /// Words for the Edit page's operations that take them.
    pub prompt: String,
    /// Edit choices, by name (`factor`, `left`...).
    pub params: BTreeMap<String, serde_json::Value>,
    pub tests: BTreeMap<String, KeyTest>,
    /// The pictures coming are traced into curves (Create vectors).
    pub trace_next: bool,
    jobs: Vec<Job>,
    key_jobs: Vec<(String, Receiver<Result<String, String>>)>,
    total: usize,
    pub transport: Arc<dyn Transport>,
    /// Tests: run each request at once on this thread.
    pub sync: bool,
    catalog_fetch: Option<Receiver<crate::net::HttpResult>>,
}

impl Default for Studio {
    fn default() -> Self {
        Studio {
            catalog: Catalog::load(),
            keys: BTreeMap::new(),
            op: Op::Upscale,
            prompt: String::new(),
            params: BTreeMap::new(),
            tests: BTreeMap::new(),
            trace_next: false,
            jobs: Vec::new(),
            key_jobs: Vec::new(),
            total: 0,
            transport: Arc::new(Net),
            sync: false,
            catalog_fetch: None,
        }
    }
}

impl Studio {
    pub fn busy(&self) -> bool {
        !self.jobs.is_empty()
    }
}

/// Run `f` on a worker thread (or at once in tests) and deliver its
/// answer on the returned channel.
fn spawn<T: Send + 'static>(sync: bool, f: impl FnOnce() -> T + Send + 'static) -> Receiver<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    if sync || crate::files::WEB {
        let _ = tx.send(f());
    } else {
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
    }
    rx
}

impl App {
    /// The key for `provider`: the studio's, else the main AI service's
    /// when that service is the same company.
    pub fn studio_key(&self, provider: &str) -> String {
        if let Some(k) = self.studio.keys.get(provider).filter(|k| !k.trim().is_empty()) {
            return k.clone();
        }
        let main = crate::ai::preset_of(&self.settings.ai)
            .and_then(|i| crate::ai::PRESETS.get(i))
            .and_then(|(k, _, _)| super::provider_of_preset(k));
        if main == Some(provider) {
            return self.ai.key.clone();
        }
        String::new()
    }

    /// Keep `key` for `provider` (and in the preferences when keys are
    /// remembered).
    pub fn studio_set_key(&mut self, provider: &str, key: String) {
        if self.settings.studio.remember_keys {
            if key.trim().is_empty() {
                self.settings.studio.keys.remove(provider);
            } else {
                self.settings.studio.keys.insert(provider.to_string(), key.clone());
            }
        }
        // The main AI service's key, when it is the same company.
        let main = crate::ai::preset_of(&self.settings.ai)
            .and_then(|i| crate::ai::PRESETS.get(i))
            .and_then(|(k, _, _)| super::provider_of_preset(k));
        if main == Some(provider) {
            self.ai.key = key.clone();
            if self.settings.ai.remember_key {
                self.settings.ai.key = key.clone();
            }
        }
        if key.trim().is_empty() {
            self.studio.keys.remove(provider);
        } else {
            self.studio.keys.insert(provider.to_string(), key);
        }
        self.studio.tests.remove(provider);
    }

    /// Remember keys on this computer or not.
    pub fn studio_remember_keys(&mut self, on: bool) {
        self.settings.studio.remember_keys = on;
        self.settings.studio.keys = if on {
            self.studio.keys.clone()
        } else {
            BTreeMap::new()
        };
        self.settings.save();
    }

    /// A picture service of the studio is chosen (not the main AI
    /// service of Tools > Options > AI).
    pub fn studio_active(&self) -> bool {
        !self.settings.studio.image_provider.is_empty()
            && self
                .studio
                .catalog
                .provider(&self.settings.studio.image_provider)
                .is_some()
    }

    /// The studio's picture service, model, shape and size for Generate
    /// and Remix.
    pub fn studio_model(&self, op: Op) -> Option<(String, super::ModelInfo)> {
        let vector = self.ai_image.mode == crate::ai_image::Mode::Vector;
        let p = self
            .studio
            .catalog
            .provider(&self.settings.studio.image_provider)?;
        let wanted = self
            .settings
            .studio
            .models
            .get(&p.id)
            .cloned()
            .unwrap_or_default();
        // The one picked, else the newest of the kind the page makes
        // (vectors on Create vectors, pictures elsewhere).
        let can = p.models_for(op);
        let m = p
            .model(&wanted)
            .filter(|m| m.ops.iter().any(|o| o == op.id()))
            .or_else(|| can.iter().copied().find(|m| m.newest && m.vector == vector))
            .or_else(|| can.iter().copied().find(|m| m.vector == vector))
            .or_else(|| can.iter().copied().find(|m| m.newest))
            .or_else(|| can.first().copied())?;
        Some((p.id.clone(), m.clone()))
    }

    /// The service chosen for an edit operation: the one picked for it,
    /// else the picture service when it does it, else the first that
    /// does.
    pub fn studio_op_service(&self, op: Op) -> Option<String> {
        let c = &self.studio.catalog;
        let can = c.services_for(op);
        let picked = self.settings.studio.op_providers.get(op.id());
        picked
            .filter(|p| can.iter().any(|s| &s.id == *p))
            .cloned()
            .or_else(|| {
                let main = &self.settings.studio.image_provider;
                can.iter().find(|s| &s.id == main).map(|s| s.id.clone())
            })
            .or_else(|| can.first().map(|s| s.id.clone()))
    }

    /// The variant chosen for `op` with `provider`.
    pub fn studio_variant(&self, provider: &str, op: Op) -> Option<super::Variant> {
        let info = self.studio.catalog.provider(provider)?.op(op)?;
        let wanted = self
            .settings
            .studio
            .variants
            .get(&format!("{provider}/{}", op.id()))
            .cloned()
            .unwrap_or_default();
        info.pick(&wanted).cloned()
    }

    /// Why a studio request cannot go now, if it cannot.
    fn studio_problem(&self, provider: &str) -> Option<String> {
        if !self.ai_features_on() {
            Some(tr("aiimg.off"))
        } else if !self.has_document() {
            Some(tr("ai.need_document"))
        } else if self.studio_key(provider).trim().is_empty() {
            let name = self
                .studio
                .catalog
                .provider(provider)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| provider.to_string());
            Some(trf("studio.need_key", &[("name", &name)]))
        } else {
            None
        }
    }

    fn studio_send(&mut self, mut req: OpRequest, target: Target) {
        let key = self.studio_key(&req.provider);
        req.english = self
            .studio
            .catalog
            .provider(&req.provider)
            .is_some_and(|p| p.english_only);
        if req.english && self.ai_ready() {
            req.translator = Some((self.settings.ai.clone(), self.ai.key.clone()));
        }
        let t = self.studio.transport.clone();
        let rx = spawn(self.studio.sync, move || super::run(t.as_ref(), &key, &req));
        self.studio.jobs.push(Job { rx, target });
    }

    fn studio_begin(&mut self, total: usize) {
        self.studio.total = total;
        self.ai_image.made = 0;
        self.ai_image.errors.clear();
        self.ai_image.status = ImageStatus::Waiting { done: 0, total };
    }

    /// Generate or Remix with the studio's picture service, from the AI
    /// Generate docker's words, style, palette, shape, size and count.
    /// `vector` traces the pictures (Create vectors).
    pub fn studio_create(&mut self, op: Op, vector: bool) {
        if self.studio.busy() || self.ai_image.busy() {
            return;
        }
        let Some((provider, model)) = self.studio_model(op) else {
            self.ai_image.status = ImageStatus::Failed(tr("studio.no_model"));
            return;
        };
        if let Some(p) = self.studio_problem(&provider) {
            self.ai_image.status = ImageStatus::Failed(p);
            return;
        }
        let (words, source) = match op {
            Op::Remix => {
                if self.ai_image.source.is_none() {
                    self.ai_source_from_selection();
                }
                let Some(src) = self.ai_image.source.clone() else {
                    self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_bitmap"));
                    return;
                };
                (self.ai_image.remix_prompt.trim().to_string(), Some(src))
            }
            _ => (
                self.ai_image.prompt.trim().to_string(),
                self.ai_image.source.clone(),
            ),
        };
        if words.is_empty() {
            self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_prompt"));
            return;
        }
        let prompt = crate::ai_image::compose_prompt(
            &words,
            if vector { self.ai_image.vector_style } else { self.ai_image.style },
            self.ai_image.palette,
            vector,
        );
        let aspect = match (self.ai_image.aspect, &source) {
            (crate::ai_image::Aspect::MatchSource, Some(s)) => {
                crate::ai_image::Aspect::nearest(s.width, s.height).ratio().to_string()
            }
            _ => self.settings.studio.aspect.clone(),
        };
        let mut params = BTreeMap::new();
        for p in &model.params {
            let value = self
                .studio
                .params
                .get(&model_param_key(&provider, &model.id, &p.name))
                .cloned()
                .unwrap_or_else(|| p.default.clone());
            params.insert(p.name.clone(), value);
        }
        let req = OpRequest {
            op: Some(op),
            provider: provider.clone(),
            model: model.id.clone(),
            params,
            prompt,
            image: source.as_ref().map(|s| s.png.clone()),
            image_size: source.as_ref().map(|s| (s.width, s.height)).unwrap_or((0, 0)),
            aspect: model.fit_aspect(&aspect),
            resolution: model.fit_resolution(&self.settings.studio.resolution),
            count: 1,
            ..Default::default()
        };
        let count = self.ai_image.count.clamp(1, crate::ai_image::MAX_COUNT) as usize;
        self.studio_begin(count);
        self.ai_image.results.clear();
        let target = match (op, &source) {
            (Op::Remix, Some(s)) => s.shape.map(Target::Over).unwrap_or(Target::New),
            _ => Target::New,
        };
        let target = if vector && !model.vector {
            // Pictures traced into curves once they are in.
            self.studio.trace_next = true;
            target
        } else {
            target
        };
        for _ in 0..count {
            self.studio_send(req.clone(), target);
        }
        self.poll_studio();
    }

    /// Run the Edit page's operation on the selected bitmap.
    pub fn studio_edit(&mut self) {
        let op = self.studio.op;
        if self.studio.busy() || self.ai_image.busy() {
            return;
        }
        let Some(provider) = self.studio_op_service(op) else {
            self.ai_image.status = ImageStatus::Failed(tr("studio.no_service"));
            return;
        };
        if let Some(p) = self.studio_problem(&provider) {
            self.ai_image.status = ImageStatus::Failed(p);
            return;
        }
        let Some((shape, png)) = self.ai_source_bitmap() else {
            self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_bitmap"));
            return;
        };
        let size = crate::ai_image::to_png(&png)
            .map(|(_, w, h)| (w, h))
            .unwrap_or((0, 0));
        if op.takes_prompt() && self.studio.prompt.trim().is_empty() {
            self.ai_image.status = ImageStatus::Failed(tr("aiimg.need_prompt"));
            return;
        }
        let variant = self.studio_variant(&provider, op);
        let mut params = BTreeMap::new();
        if let Some(v) = &variant {
            for p in &v.params {
                let value = self
                    .studio
                    .params
                    .get(&op_param_key(&provider, op, &v.id, &p.name))
                    .cloned()
                    .unwrap_or_else(|| p.default.clone());
                params.insert(p.name.clone(), value);
            }
        }
        let target = match op {
            Op::Expand => {
                let px = |name: &str| {
                    params
                        .get(name)
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0)
                        .max(0.0) as u32
                };
                let (left, right, top, bottom) = (px("left"), px("right"), px("top"), px("bottom"));
                if left + right + top + bottom == 0 {
                    self.ai_image.status = ImageStatus::Failed(tr("studio.expand_nothing"));
                    return;
                }
                Target::Expand { shape, left, right, top, bottom }
            }
            Op::RemoveBackground => Target::Cutout(shape),
            Op::Vectorize => Target::Vector(shape),
            _ => Target::Replace(shape),
        };
        let req = OpRequest {
            op: Some(op),
            provider,
            model: String::new(),
            variant: variant.map(|v| v.id).unwrap_or_default(),
            prompt: self.studio.prompt.trim().to_string(),
            image: Some(png),
            image_size: size,
            count: 1,
            params,
            ..Default::default()
        };
        self.studio_begin(1);
        self.status = tr(op.key());
        self.studio_send(req, target);
        self.poll_studio();
    }

    /// Take in finished requests and key tests. True while some run.
    pub fn poll_studio(&mut self) -> bool {
        let mut i = 0;
        while i < self.studio.jobs.len() {
            match self.studio.jobs[i].rx.try_recv() {
                Ok(r) => {
                    let job = self.studio.jobs.remove(i);
                    self.studio_receive(r, job.target);
                }
                Err(TryRecvError::Empty) => i += 1,
                Err(TryRecvError::Disconnected) => {
                    let job = self.studio.jobs.remove(i);
                    self.studio_receive(Err(tr("ai.connection_lost")), job.target);
                }
            }
        }
        let mut i = 0;
        while i < self.studio.key_jobs.len() {
            let got = match self.studio.key_jobs[i].1.try_recv() {
                Ok(r) => Some(r),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(tr("ai.connection_lost"))),
            };
            match got {
                Some(r) => {
                    let (provider, _) = self.studio.key_jobs.remove(i);
                    self.studio.tests.insert(
                        provider,
                        match r {
                            Ok(note) => KeyTest::Works(note),
                            Err(e) => KeyTest::Fails(e),
                        },
                    );
                }
                None => i += 1,
            }
        }
        if let Some(rx) = &self.studio.catalog_fetch {
            match rx.try_recv() {
                Ok(r) => {
                    self.studio.catalog_fetch = None;
                    self.studio_take_catalog(r);
                }
                Err(TryRecvError::Disconnected) => self.studio.catalog_fetch = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        self.studio.busy() || !self.studio.key_jobs.is_empty() || self.studio.catalog_fetch.is_some()
    }

    /// Stop waiting (answers are dropped when they come).
    pub fn studio_cancel(&mut self) {
        if self.studio.busy() {
            self.studio.jobs.clear();
            self.studio.trace_next = false;
            self.ai_image.status = ImageStatus::Idle;
        }
    }

    /// Test the key of `provider` in the background.
    pub fn studio_test_key(&mut self, provider: &str) {
        let key = self.studio_key(provider);
        let t = self.studio.transport.clone();
        let p = provider.to_string();
        let rx = spawn(self.studio.sync, move || super::test_key(t.as_ref(), &p, &key));
        self.studio.tests.insert(provider.to_string(), KeyTest::Testing);
        self.studio.key_jobs.push((provider.to_string(), rx));
        self.poll_studio();
    }

    /// Ask the edition's repository for a newer catalog (at most once a
    /// day, at start).
    pub fn studio_refresh_catalog(&mut self) {
        if crate::files::WEB || self.studio.catalog_fetch.is_some() {
            return;
        }
        let now = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if now.saturating_sub(self.settings.studio_catalog_checked) < 24 * 60 * 60 {
            return;
        }
        self.settings.studio_catalog_checked = now;
        let t = self.studio.transport.clone();
        let req = super::transport::get(super::catalog::REMOTE_URL.into(), Vec::new());
        self.studio.catalog_fetch = Some(spawn(self.studio.sync, move || t.call(&req)));
    }

    fn studio_take_catalog(&mut self, r: crate::net::HttpResult) {
        let Ok(r) = r else { return };
        if r.status != 200 {
            return;
        }
        if let Some(c) = self.studio.catalog.newer(&r.body) {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(dir) = crate::settings::config_dir() {
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join(super::catalog::CACHE_FILE), &r.body);
            }
            self.studio.catalog = c;
        }
    }

    fn studio_receive(&mut self, result: Result<Output, String>, target: Target) {
        let done = self.studio.total.saturating_sub(self.studio.jobs.len());
        match result {
            Err(e) => self.ai_image.errors.push(e),
            Ok(Output::Svg(svg)) => {
                let over = match target {
                    Target::Vector(s) | Target::Over(s) | Target::Replace(s) => Some(s),
                    _ => None,
                };
                self.studio_place_svg(&svg, over);
            }
            Ok(Output::Images(files)) => {
                let images: Vec<GenImage> = files
                    .iter()
                    .filter_map(|f| crate::ai_image::to_png(f))
                    .map(|(png, width, height)| GenImage { png, width, height })
                    .collect();
                if images.is_empty() {
                    self.ai_image.errors.push(tr("aiimg.unreadable_image"));
                }
                for img in images {
                    self.studio_apply(img, target);
                }
            }
        }
        self.ai_image.status = if self.studio.busy() {
            ImageStatus::Waiting { done, total: self.studio.total }
        } else {
            self.studio.trace_next = false;
            if self.ai_image.made == 0 {
                ImageStatus::Failed(
                    self.ai_image
                        .errors
                        .first()
                        .cloned()
                        .unwrap_or_else(|| tr("aiimg.no_image")),
                )
            } else {
                ImageStatus::Done(self.ai_image.made)
            }
        };
    }

    fn studio_apply(&mut self, img: GenImage, target: Target) {
        match target {
            Target::New => {
                let trace = self.studio.trace_next;
                if self.ai_image.auto_place || trace {
                    self.ai_place_image(&img, None, trace);
                } else {
                    self.ai_image.made += 1;
                }
                self.ai_keep_result(img);
            }
            Target::Over(shape) => {
                if self.ai_image.auto_place {
                    self.ai_place_image(&img, Some(shape), false);
                } else {
                    self.ai_image.made += 1;
                }
                self.ai_keep_result(img);
            }
            Target::Cutout(shape) => self.ai_apply_cutout(shape, img, false),
            Target::Replace(shape) => self.studio_replace_pixels(shape, img, None),
            Target::Expand { shape, left, right, top, bottom } => {
                self.studio_replace_pixels(shape, img, Some((left, right, top, bottom)))
            }
            Target::Vector(shape) => {
                // A service that answers vectorize with pixels: trace them.
                self.studio_replace_pixels(shape, img, None);
            }
        }
    }

    /// The picture `shape` with new pixels: the same frame, or one grown
    /// by `grow` (left, right, top, bottom, in the old picture's pixels).
    fn studio_replace_pixels(&mut self, shape: ShapeId, img: GenImage, grow: Option<(u32, u32, u32, u32)>) {
        let Some(src) = self.doc().find_shape(shape).cloned() else {
            self.ai_image.errors.push(tr("aiimg.source_gone"));
            return;
        };
        let ShapeKind::Bitmap { rect, width_px, height_px, .. } = &src.kind else {
            self.ai_image.errors.push(tr("aiimg.source_gone"));
            return;
        };
        let rect = match grow {
            Some(g) => grown_rect(*rect, (*width_px, *height_px), g),
            None => *rect,
        };
        let cmd = Command::SetShapeKind {
            shape,
            kind: ShapeKind::Bitmap {
                rect,
                width_px: img.width,
                height_px: img.height,
                png: img.png.clone(),
                fx: None,
            },
        };
        match self.engine.run_batch(LABEL_STUDIO, &[cmd]) {
            Ok(_) => {
                self.ai_image.made += 1;
                self.ai_keep_result(img);
                self.status = tr("studio.done");
            }
            Err(e) => self.ai_image.errors.push(e.to_string()),
        }
    }

    /// Curves from an SVG: over the picture `over` (fitted to its frame,
    /// the picture left in place, hidden under them) or centred on the
    /// page.
    fn studio_place_svg(&mut self, svg: &str, over: Option<ShapeId>) {
        use tracedraw_core::geometry::Shape as _;
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed =
            tracedraw_io::svg_import::parse(svg, &mut tracedraw_core::id::IdSource::default());
        let shapes = match parsed {
            Ok(p) if !p.shapes.is_empty() => p.shapes,
            Ok(_) => {
                self.ai_image.errors.push(tr("aiimg.no_svg"));
                return;
            }
            Err(e) => {
                self.ai_image.errors.push(trf("aiimg.bad_svg", &[("e", &e)]));
                return;
            }
        };
        let gid = self.engine.new_shape_id();
        let children: Vec<Shape> = shapes
            .into_iter()
            .map(|s| {
                let id = self.engine.new_shape_id();
                crate::app::reid_pub(s, id, &mut self.engine)
            })
            .collect();
        let mut group = Shape::new(gid, ShapeKind::Group { children });
        group.name = Some(tr("aiimg.vector_name"));
        let b = group.page_path().bounding_box();
        let frame = over
            .and_then(|id| self.doc().find_shape(id).cloned())
            .map(|s| s.page_path().bounding_box());
        if b.width() > 1e-9 && b.height() > 1e-9 {
            let (cx, cy, k) = match frame {
                Some(f) if f.width() > 1e-9 => {
                    let k = (f.width() / b.width()).min(f.height() / b.height());
                    (f.center().x, f.center().y, k)
                }
                _ => {
                    let page = self.page_size();
                    let k = (page.width * 0.8 / b.width()).min(page.height * 0.8 / b.height());
                    (page.width / 2.0, page.height / 2.0, k)
                }
            };
            let c = b.center();
            group.transform = tracedraw_core::Affine::translate(tracedraw_core::Vec2::new(cx, cy))
                * tracedraw_core::Affine::scale(k)
                * tracedraw_core::Affine::translate(-c.to_vec2());
        }
        if let Err(e) = self.engine.run_batch(
            LABEL_STUDIO,
            &[Command::AddShape { layer, shape: group }],
        ) {
            self.ai_image.errors.push(e.to_string());
            return;
        }
        self.ai_image.made += 1;
        self.select(vec![gid]);
        self.status = tr("studio.done");
    }
}

/// Where a model's choice is kept in `Studio::params`.
pub fn model_param_key(provider: &str, model: &str, name: &str) -> String {
    format!("{provider}.{model}.{name}")
}

/// Where an operation's choice is kept in `Studio::params`.
pub fn op_param_key(provider: &str, op: Op, variant: &str, name: &str) -> String {
    format!("{provider}.{}.{variant}.{name}", op.id())
}

/// The frame of a picture of `px` pixels in `rect` once it has grown by
/// `(left, right, top, bottom)` of those pixels (page y up: top is y1).
pub fn grown_rect(rect: Rect, px: (u32, u32), grow: (u32, u32, u32, u32)) -> Rect {
    let mx = rect.width() / px.0.max(1) as f64;
    let my = rect.height() / px.1.max(1) as f64;
    let (l, r, t, b) = grow;
    Rect::new(
        rect.x0 - l as f64 * mx,
        rect.y0 - b as f64 * my,
        rect.x1 + r as f64 * mx,
        rect.y1 + t as f64 * my,
    )
}

#[cfg(test)]
mod tests {
    use super::super::transport::{reply, Fake};
    use super::*;
    use serde_json::json;
    use tracedraw_core::document::ShapeKind;

    /// A service that answers every picture request with a 64 x 48 PNG
    /// and vectorize with an SVG.
    fn answer(r: &crate::net::HttpRequest) -> crate::net::HttpResult {
        if r.url.ends_with("/vectorize") {
            let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\" fill=\"#f00\"/></svg>";
            return reply(200, json!({"image": {"image_id": "1", "b64_json": super::super::transport::base64(svg)}}));
        }
        let png = super::super::transport::base64(&super::super::test_png(64, 48));
        if r.url.contains("recraft") {
            if r.url.ends_with("/generations") || r.url.ends_with("/imageToImage") || r.url.ends_with("/outpaint") {
                return reply(200, json!({"data": [{"image_id": "1", "b64_json": png}]}));
            }
            return reply(200, json!({"image": {"image_id": "1", "b64_json": png}}));
        }
        reply(500, json!({"message": "unexpected"}))
    }

    fn app() -> App {
        let mut app = App::headless();
        app.studio.sync = true;
        app.studio.transport = Arc::new(Fake::new(answer, |_| Err("no".into())));
        app.studio_set_key("recraft", "rk".into());
        app.settings.studio.image_provider = "recraft".into();
        app
    }

    fn bitmap(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: Rect::new(10.0, 10.0, 42.0, 34.0),
                width_px: 320,
                height_px: 240,
                png: super::super::test_png(320, 240),
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        id
    }

    fn kind(app: &App, id: ShapeId) -> ShapeKind {
        app.doc().find_shape(id).map(|s| s.kind.clone()).expect("shape")
    }

    #[test]
    fn created_pictures_come_from_the_studio_service() {
        let mut app = app();
        assert!(app.studio_active());
        app.ai_image.prompt = "a fox".into();
        app.ai_image.count = 2;
        app.ai_generate();
        assert_eq!(app.ai_image.status, ImageStatus::Done(2), "{:?}", app.ai_image.status);
        let n = app
            .doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter())
            .filter(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
            .count();
        assert_eq!(n, 2);
    }

    #[test]
    fn upscale_replaces_the_pixels_in_the_same_frame() {
        let mut app = app();
        let id = bitmap(&mut app);
        app.studio.op = Op::Upscale;
        app.studio_edit();
        assert_eq!(app.ai_image.status, ImageStatus::Done(1), "{:?}", app.ai_image.status);
        let ShapeKind::Bitmap { rect, width_px, .. } = kind(&app, id) else { panic!("bitmap") };
        assert_eq!((rect, width_px), (Rect::new(10.0, 10.0, 42.0, 34.0), 64));
        app.undo();
        let ShapeKind::Bitmap { width_px, .. } = kind(&app, id) else { panic!("bitmap") };
        assert_eq!(width_px, 320, "one undo step");
    }

    #[test]
    fn expand_grows_the_frame_by_the_pixels_added() {
        let mut app = app();
        let id = bitmap(&mut app);
        app.studio.op = Op::Expand;
        app.studio.params.insert(op_param_key("recraft", Op::Expand, "outpaint", "right"), json!(160));
        app.studio.params.insert(op_param_key("recraft", Op::Expand, "outpaint", "top"), json!(120));
        app.studio_edit();
        assert_eq!(app.ai_image.status, ImageStatus::Done(1), "{:?}", app.ai_image.status);
        let ShapeKind::Bitmap { rect, .. } = kind(&app, id) else { panic!("bitmap") };
        // 320 px over 32 mm: 0.1 mm a pixel.
        assert!((rect.x1 - 58.0).abs() < 1e-6 && (rect.y1 - 46.0).abs() < 1e-6 && rect.x0 == 10.0 && rect.y0 == 10.0, "{rect:?}");
        // Nothing to add: refused before anything is sent.
        app.studio.params.clear();
        app.studio_edit();
        assert_eq!(app.ai_image.status, ImageStatus::Failed(tr("studio.expand_nothing")));
    }

    #[test]
    fn vectors_go_over_the_picture_and_a_missing_key_is_said() {
        let mut app = app();
        bitmap(&mut app);
        app.studio.op = Op::Vectorize;
        app.studio_edit();
        assert_eq!(app.ai_image.status, ImageStatus::Done(1), "{:?}", app.ai_image.status);
        let sel = app.selected_shapes();
        assert!(matches!(sel.first().map(|s| &s.kind), Some(ShapeKind::Group { .. })));
        // Another service without a key.
        app.settings.studio.op_providers.insert("upscale".into(), "stability".into());
        app.studio.op = Op::Upscale;
        bitmap(&mut app);
        app.studio_edit();
        assert!(matches!(&app.ai_image.status, ImageStatus::Failed(e) if e.contains("Stability")));
    }

    #[test]
    fn keys_are_kept_by_company_and_shared_with_the_main_service() {
        let mut app = app();
        // The main service is Anthropic by default: its key is the studio's.
        app.studio_set_key("anthropic", "ak".into());
        assert_eq!(app.ai.key, "ak");
        assert_eq!(app.studio_key("anthropic"), "ak");
        assert_eq!(app.studio_key("recraft"), "rk");
        assert_eq!(app.settings.studio.keys.get("recraft").map(String::as_str), Some("rk"));
        app.studio_remember_keys(false);
        assert!(app.settings.studio.keys.is_empty());
        assert_eq!(app.studio_key("recraft"), "rk", "still in use this session");
    }
}
