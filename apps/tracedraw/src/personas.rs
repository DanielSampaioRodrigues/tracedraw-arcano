//! Arcano edition: workspaces one click apart, at the right of the menu
//! bar: Drawing, Page and the user's own. Each keeps its own panels:
//! leaving one keeps how its panels were, and coming back puts them back;
//! the first time, Page lays out the panels a book's pages need (Pages and
//! Layers; Character, Paragraph and Text Styles; House Press and Align).
//! The small "+" at the end saves the panels as they are as a workspace
//! of the user's, with a name, a colour and a picture. Every workspace
//! has a colour (Drawing green, Page orange): while it is in use the
//! interface's accent takes it. A right click on a workspace changes its
//! name, colour and picture, puts its panels back or deletes it (the
//! user's only). The drawing, the tools and the menus stay the same. See
//! docs/behavior/personas.md.

use crate::app::{App, DockerTab};
use crate::settings::{DockerSetup, StackGroupSetup};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persona {
    Drawing,
    Page,
}

/// Drawing's colour: the program's green.
pub const DRAWING_COLOR: [u8; 3] = crate::theme::GREEN;
/// Page's colour: orange.
pub const PAGE_COLOR: [u8; 3] = [0xE0, 0x6C, 0x12];

/// The colours offered for a workspace.
pub const COLORS: [[u8; 3]; 12] = [
    DRAWING_COLOR,
    PAGE_COLOR,
    [0x1E, 0x78, 0xD2],
    [0x6F, 0x4C, 0xD8],
    [0xCF, 0x3A, 0x3A],
    [0xD1, 0x45, 0x8C],
    [0x12, 0x92, 0x8C],
    [0xC2, 0x93, 0x0A],
    [0x95, 0x5E, 0x33],
    [0x58, 0x60, 0x6E],
    [0x2F, 0x9E, 0xC9],
    [0x7E, 0x9C, 0x1D],
];

/// The pictures offered for a workspace, by name.
pub const ICONS: [&str; 16] = [
    "pen", "page", "photo", "brush", "type", "book", "star", "grid", "palette", "heart", "leaf",
    "layers", "scissors", "cup", "bolt", "none",
];

impl Persona {
    pub const ALL: [Persona; 2] = [Persona::Drawing, Persona::Page];

    pub fn id(self) -> &'static str {
        match self {
            Persona::Drawing => "drawing",
            Persona::Page => "page",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Persona::Drawing => "persona.drawing",
            Persona::Page => "persona.page",
        }
    }

    pub fn from_id(id: &str) -> Persona {
        match id {
            "page" => Persona::Page,
            _ => Persona::Drawing,
        }
    }

    /// The panels it starts with.
    pub fn preset(self) -> DockerSetup {
        let k = |t: DockerTab| t.key().to_string();
        let group = |tabs: &[DockerTab], weight: f32| StackGroupSetup {
            tabs: tabs.iter().map(|t| k(*t)).collect(),
            front: k(tabs[0]),
            weight,
        };
        let groups: Vec<StackGroupSetup> = match self {
            Persona::Drawing => vec![
                group(&[DockerTab::Properties, DockerTab::Hints], 1.4),
                group(&[DockerTab::Objects], 1.0),
            ],
            Persona::Page => vec![
                group(&[DockerTab::Pages, DockerTab::Objects], 1.0),
                group(
                    &[
                        DockerTab::Character,
                        DockerTab::Paragraph,
                        DockerTab::TextStyles,
                    ],
                    1.5,
                ),
                group(&[DockerTab::HousePress, DockerTab::AlignDistribute], 1.0),
            ],
        };
        let open: Vec<String> = groups.iter().flat_map(|g| g.tabs.clone()).collect();
        DockerSetup {
            front: open[0].clone(),
            open,
            stacks: vec![groups],
            ..Default::default()
        }
    }
}

/// A workspace of the user's, kept in the preferences.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UserWorkspace {
    pub id: String,
    pub name: String,
    pub color: [u8; 3],
    pub icon: String,
    /// A picture of the user's ("custom" icon): a square alpha mask, see
    /// [`ICON_SIDE`], as base64.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mask: String,
    /// The panels it was saved with (Restore puts them back).
    pub setup: DockerSetup,
}

/// The colour and picture a built-in workspace was given.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceLook {
    pub color: Option<[u8; 3]>,
    pub icon: Option<String>,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mask: String,
}

/// A workspace of the switch, built-in or the user's.
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub color: [u8; 3],
    pub icon: String,
    /// The user's own picture, for icon "custom".
    pub mask: String,
    pub builtin: bool,
}

impl App {
    /// The built-in workspace in use (Drawing for one of the user's).
    #[cfg(test)]
    pub fn persona(&self) -> Persona {
        Persona::from_id(&self.settings.arcano_persona)
    }

    /// The id of the workspace in use.
    pub fn workspace_id(&self) -> String {
        let id = &self.settings.arcano_persona;
        if self.workspaces().iter().any(|w| w.id == *id) {
            id.clone()
        } else {
            Persona::Drawing.id().to_string()
        }
    }

    /// Every workspace of the switch: Drawing, Page, then the user's.
    pub fn workspaces(&self) -> Vec<Workspace> {
        let mut out: Vec<Workspace> = Persona::ALL
            .iter()
            .map(|p| {
                let look = self.settings.arcano_workspace_looks.get(p.id());
                let (color, icon) = match p {
                    Persona::Drawing => (DRAWING_COLOR, "pen"),
                    Persona::Page => (PAGE_COLOR, "page"),
                };
                Workspace {
                    id: p.id().to_string(),
                    name: look
                        .and_then(|l| l.name.clone())
                        .filter(|n| !n.trim().is_empty())
                        .unwrap_or_else(|| crate::i18n::tr(p.key())),
                    color: look.and_then(|l| l.color).unwrap_or(color),
                    icon: look
                        .and_then(|l| l.icon.clone())
                        .unwrap_or_else(|| icon.to_string()),
                    mask: look.map(|l| l.mask.clone()).unwrap_or_default(),
                    builtin: true,
                }
            })
            .collect();
        out.extend(self.settings.arcano_workspaces.iter().map(|w| Workspace {
            id: w.id.clone(),
            name: w.name.clone(),
            color: w.color,
            icon: w.icon.clone(),
            mask: w.mask.clone(),
            builtin: false,
        }));
        out
    }

    /// The colour of the workspace in use: the interface's accent.
    pub fn workspace_color(&self) -> [u8; 3] {
        let id = self.workspace_id();
        self.workspaces()
            .into_iter()
            .find(|w| w.id == id)
            .map_or(DRAWING_COLOR, |w| w.color)
    }

    /// The panels workspace `id` starts with.
    fn workspace_preset(&self, id: &str) -> DockerSetup {
        match self.settings.arcano_workspaces.iter().find(|w| w.id == id) {
            Some(w) => w.setup.clone(),
            None => Persona::from_id(id).preset(),
        }
    }

    /// Change to the built-in workspace `to`.
    pub fn switch_persona(&mut self, to: Persona) {
        self.switch_workspace(to.id());
    }

    /// Change to workspace `id`: the panels of the one left are kept, and
    /// `id`'s come back (its preset the first time).
    pub fn switch_workspace(&mut self, id: &str) {
        let from = self.workspace_id();
        if from == id || !self.workspaces().iter().any(|w| w.id == id) {
            return;
        }
        let now = self.docker_setup();
        self.settings.arcano_persona_layouts.insert(from, now);
        let setup = self
            .settings
            .arcano_persona_layouts
            .get(id)
            .cloned()
            .unwrap_or_else(|| self.workspace_preset(id));
        self.set_docker_setup(&setup);
        self.show_dockers = true;
        if id == Persona::Page.id() {
            self.show_rulers = true;
            self.show_guides = true;
        }
        self.stack_flyout = None;
        self.settings.arcano_persona = id.to_string();
        let name = self
            .workspaces()
            .into_iter()
            .find(|w| w.id == id)
            .map(|w| w.name)
            .unwrap_or_default();
        self.status = crate::i18n::trf("persona.switched", &[("name", &name)]);
    }

    /// Back to the panels the workspace in use starts with.
    pub fn reset_persona(&mut self) {
        let id = self.workspace_id();
        self.settings.arcano_persona_layouts.remove(&id);
        let setup = self.workspace_preset(&id);
        self.set_docker_setup(&setup);
    }

    /// Save the panels as they are as a new workspace of the user's, and
    /// use it. Returns its id.
    #[cfg(test)]
    pub fn save_workspace(&mut self, name: &str, color: [u8; 3], icon: &str) -> String {
        self.save_workspace_with(name, color, icon, "")
    }

    /// [`App::save_workspace`] with a picture of the user's.
    pub fn save_workspace_with(&mut self, name: &str, color: [u8; 3], icon: &str, mask: &str) -> String {
        let n = (1..)
            .find(|n| {
                !self
                    .settings
                    .arcano_workspaces
                    .iter()
                    .any(|w| w.id == format!("user-{n}"))
            })
            .unwrap_or(1);
        let id = format!("user-{n}");
        let name = if name.trim().is_empty() {
            crate::i18n::trf("persona.new_name", &[("n", &n.to_string())])
        } else {
            name.trim().to_string()
        };
        let setup = self.docker_setup();
        let from = self.workspace_id();
        self.settings
            .arcano_persona_layouts
            .insert(from, setup.clone());
        self.settings.arcano_workspaces.push(UserWorkspace {
            id: id.clone(),
            name: name.clone(),
            color,
            icon: icon.to_string(),
            mask: mask.to_string(),
            setup: setup.clone(),
        });
        self.settings
            .arcano_persona_layouts
            .insert(id.clone(), setup);
        self.settings.arcano_persona = id.clone();
        self.status = crate::i18n::trf("persona.saved", &[("name", &name)]);
        id
    }

    /// The panels as they are become what workspace `id` starts with.
    pub fn resave_workspace(&mut self, id: &str) {
        let setup = self.docker_setup();
        if let Some(w) = self.settings.arcano_workspaces.iter_mut().find(|w| w.id == id) {
            w.setup = setup.clone();
        }
        self.settings.arcano_persona_layouts.insert(id.to_string(), setup);
    }

    /// Give workspace `id` a name, a colour and a picture.
    #[cfg(test)]
    pub fn set_workspace_look(&mut self, id: &str, name: &str, color: [u8; 3], icon: &str) {
        self.set_workspace_look_with(id, name, color, icon, "");
    }

    /// [`App::set_workspace_look`] with a picture of the user's.
    pub fn set_workspace_look_with(&mut self, id: &str, name: &str, color: [u8; 3], icon: &str, mask: &str) {
        if let Some(w) = self.settings.arcano_workspaces.iter_mut().find(|w| w.id == id) {
            if !name.trim().is_empty() {
                w.name = name.trim().to_string();
            }
            w.color = color;
            w.icon = icon.to_string();
            w.mask = mask.to_string();
            return;
        }
        if Persona::ALL.iter().any(|p| p.id() == id) {
            let look = self
                .settings
                .arcano_workspace_looks
                .entry(id.to_string())
                .or_default();
            look.color = Some(color);
            look.icon = Some(icon.to_string());
            look.mask = mask.to_string();
            look.name = Some(name.trim().to_string()).filter(|n| !n.is_empty());
        }
    }

    /// Delete a workspace of the user's (Drawing comes back when it was
    /// in use).
    pub fn delete_workspace(&mut self, id: &str) {
        if !self.settings.arcano_workspaces.iter().any(|w| w.id == id) {
            return;
        }
        if self.workspace_id() == id {
            self.switch_persona(Persona::Drawing);
        }
        self.settings.arcano_workspaces.retain(|w| w.id != id);
        self.settings.arcano_persona_layouts.remove(id);
    }
}

// ----- pictures -------------------------------------------------------------

/// Draw the workspace picture `name` in `r` with `color`.
pub fn paint_icon(p: &egui::Painter, r: egui::Rect, name: &str, color: egui::Color32) {
    use egui::{pos2, Shape, Stroke};
    let s = Stroke::new(1.3, color);
    let c = r.center();
    let w = r.width().min(r.height()) / 2.0 - 1.0;
    let at = |x: f32, y: f32| pos2(c.x + x * w, c.y + y * w);
    let line = |pts: &[(f32, f32)]| Shape::line(pts.iter().map(|&(x, y)| at(x, y)).collect(), s);
    let shapes: Vec<Shape> = match name {
        "pen" => vec![
            line(&[(-0.8, 0.8), (0.5, -0.5), (0.8, -0.2), (-0.5, 1.0), (-0.8, 0.8)]),
            line(&[(0.35, -0.65), (0.65, -0.95), (0.95, -0.65), (0.65, -0.35)]),
        ],
        "page" => vec![
            line(&[(-0.6, -0.9), (0.3, -0.9), (0.65, -0.55), (0.65, 0.9), (-0.6, 0.9), (-0.6, -0.9)]),
            line(&[(-0.35, -0.3), (0.4, -0.3)]),
            line(&[(-0.35, 0.05), (0.4, 0.05)]),
            line(&[(-0.35, 0.4), (0.2, 0.4)]),
        ],
        "photo" => vec![
            line(&[(-0.9, -0.6), (0.9, -0.6), (0.9, 0.7), (-0.9, 0.7), (-0.9, -0.6)]),
            line(&[(-0.75, 0.55), (-0.25, 0.0), (0.1, 0.35), (0.35, 0.1), (0.75, 0.55)]),
            Shape::circle_stroke(at(0.45, -0.25), w * 0.15, s),
        ],
        "brush" => vec![
            line(&[(0.9, -0.9), (-0.1, 0.2)]),
            line(&[(-0.1, 0.2), (-0.35, 0.05), (-0.75, 0.45), (-0.9, 0.9), (-0.45, 0.75), (-0.05, 0.35), (-0.1, 0.2)]),
        ],
        "type" => vec![
            line(&[(-0.7, -0.75), (0.7, -0.75)]),
            line(&[(0.0, -0.75), (0.0, 0.85)]),
            line(&[(-0.3, 0.85), (0.3, 0.85)]),
        ],
        "book" => vec![
            line(&[(0.0, -0.6), (-0.9, -0.85), (-0.9, 0.65), (0.0, 0.9), (0.9, 0.65), (0.9, -0.85), (0.0, -0.6), (0.0, 0.9)]),
        ],
        "star" => {
            let pts: Vec<(f32, f32)> = (0..11)
                .map(|i| {
                    let a = std::f32::consts::PI * (i as f32) / 5.0 - std::f32::consts::FRAC_PI_2;
                    let rr = if i % 2 == 0 { 0.95 } else { 0.42 };
                    (a.cos() * rr, a.sin() * rr + 0.05)
                })
                .collect();
            vec![line(&pts)]
        }
        "grid" => vec![
            line(&[(-0.85, -0.85), (-0.1, -0.85), (-0.1, -0.1), (-0.85, -0.1), (-0.85, -0.85)]),
            line(&[(0.1, -0.85), (0.85, -0.85), (0.85, -0.1), (0.1, -0.1), (0.1, -0.85)]),
            line(&[(-0.85, 0.1), (-0.1, 0.1), (-0.1, 0.85), (-0.85, 0.85), (-0.85, 0.1)]),
            line(&[(0.1, 0.1), (0.85, 0.1), (0.85, 0.85), (0.1, 0.85), (0.1, 0.1)]),
        ],
        "palette" => vec![
            Shape::circle_stroke(c, w * 0.9, s),
            Shape::circle_filled(at(-0.35, -0.35), w * 0.14, color),
            Shape::circle_filled(at(0.25, -0.45), w * 0.14, color),
            Shape::circle_filled(at(0.5, 0.1), w * 0.14, color),
        ],
        "heart" => vec![line(&[
            (0.0, 0.85),
            (-0.8, 0.0),
            (-0.85, -0.45),
            (-0.5, -0.8),
            (-0.1, -0.6),
            (0.0, -0.4),
            (0.1, -0.6),
            (0.5, -0.8),
            (0.85, -0.45),
            (0.8, 0.0),
            (0.0, 0.85),
        ])],
        "leaf" => vec![
            line(&[(-0.8, 0.8), (-0.6, -0.2), (0.0, -0.7), (0.85, -0.85), (0.7, 0.0), (0.2, 0.6), (-0.8, 0.8)]),
            line(&[(-0.8, 0.8), (0.4, -0.4)]),
        ],
        "layers" => vec![
            line(&[(0.0, -0.85), (0.9, -0.4), (0.0, 0.05), (-0.9, -0.4), (0.0, -0.85)]),
            line(&[(-0.9, 0.05), (0.0, 0.5), (0.9, 0.05)]),
            line(&[(-0.9, 0.45), (0.0, 0.9), (0.9, 0.45)]),
        ],
        "scissors" => vec![
            Shape::circle_stroke(at(-0.55, 0.55), w * 0.3, s),
            Shape::circle_stroke(at(0.55, 0.55), w * 0.3, s),
            line(&[(-0.35, 0.3), (0.55, -0.9)]),
            line(&[(0.35, 0.3), (-0.55, -0.9)]),
        ],
        "cup" => vec![
            line(&[(-0.7, -0.5), (0.5, -0.5), (0.4, 0.75), (-0.6, 0.75), (-0.7, -0.5)]),
            line(&[(0.5, -0.25), (0.85, -0.2), (0.85, 0.2), (0.45, 0.3)]),
            line(&[(-0.3, -0.95), (-0.2, -0.7)]),
            line(&[(0.1, -0.95), (0.2, -0.7)]),
        ],
        "bolt" => vec![line(&[(0.2, -0.95), (-0.55, 0.1), (0.0, 0.1), (-0.2, 0.95), (0.55, -0.15), (0.0, -0.15), (0.2, -0.95)])],
        _ => Vec::new(),
    };
    for sh in shapes {
        p.add(sh);
    }
}

/// The side of a picture of the user's, pixels.
pub const ICON_SIDE: u32 = 48;

/// A picture of the user's from premultiplied RGBA `w` by `h`: its
/// opaque part, fitted whole and centred in a square of [`ICON_SIDE`],
/// as base64 of one alpha byte per pixel. `None` when nothing shows.
pub fn mask_from_rgba(rgba: &[u8], w: u32, h: u32) -> Option<String> {
    use base64::Engine as _;
    if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
        return None;
    }
    let alpha = |x: u32, y: u32| rgba.get(((y * w + x) * 4 + 3) as usize).copied().unwrap_or(0);
    // The box of what shows.
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if alpha(x, y) > 8 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let side = ICON_SIDE as f32;
    let (bw, bh) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let scale = (side - 2.0) / bw.max(bh);
    let (ox, oy) = ((side - bw * scale) / 2.0, (side - bh * scale) / 2.0);
    let mut out = vec![0u8; (ICON_SIDE * ICON_SIDE) as usize];
    for (i, px) in out.iter_mut().enumerate() {
        let (tx, ty) = ((i as u32 % ICON_SIDE) as f32, (i as u32 / ICON_SIDE) as f32);
        // Four samples a pixel, taken from the source box.
        let mut sum = 0u32;
        for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
            let sx = (tx + dx - ox) / scale + x0 as f32;
            let sy = (ty + dy - oy) / scale + y0 as f32;
            if sx >= x0 as f32 && sy >= y0 as f32 && sx < x1 as f32 && sy < y1 as f32 {
                sum += alpha(sx as u32, sy as u32) as u32;
            }
        }
        *px = (sum / 4) as u8;
    }
    Some(base64::engine::general_purpose::STANDARD.encode(out))
}

/// A picture of the user's from the selected objects.
pub fn mask_from_selection(app: &App) -> Option<String> {
    let shapes = app.selected_shapes();
    if shapes.is_empty() {
        return None;
    }
    let shape = if shapes.len() == 1 {
        shapes.into_iter().next()?
    } else {
        tracedraw_core::Shape::new(tracedraw_core::ShapeId(u64::MAX - 7), tracedraw_core::ShapeKind::Group { children: shapes })
    };
    let (rgba, w, h) = crate::object_thumbs::render(app.doc(), &shape, 192)?;
    mask_from_rgba(&rgba, w, h)
}

/// A picture of the user's from an image or SVG file's bytes.
pub fn mask_from_file(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok().filter(|t| t.contains("<svg"));
    if let Some(svg) = text {
        let doc_shapes = tracedraw_io::svg_import::parse(svg, &mut tracedraw_core::id::IdSource::default()).ok()?;
        if doc_shapes.shapes.is_empty() {
            return None;
        }
        let shape = tracedraw_core::Shape::new(
            tracedraw_core::ShapeId(u64::MAX - 7),
            tracedraw_core::ShapeKind::Group { children: doc_shapes.shapes },
        );
        let doc = tracedraw_core::Document::new("icon", doc_shapes.size);
        let (rgba, w, h) = crate::object_thumbs::render(&doc, &shape, 192)?;
        return mask_from_rgba(&rgba, w, h);
    }
    let img = image::load_from_memory(bytes).ok()?;
    let img = img.thumbnail(256, 256).to_rgba8();
    let (w, h) = img.dimensions();
    // A picture with no transparency: its dark parts are the picture.
    let mut rgba = img.into_raw();
    if rgba.chunks(4).all(|p| p[3] == 255) {
        for p in rgba.chunks_mut(4) {
            let lum = (p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32) / 10;
            p[3] = 255 - lum.min(255) as u8;
        }
    }
    mask_from_rgba(&rgba, w, h)
}

/// Draw a picture of the user's in `r`, in `color`.
pub fn paint_mask(p: &egui::Painter, r: egui::Rect, mask: &str, color: egui::Color32) {
    use base64::Engine as _;
    let ctx = p.ctx();
    let id = egui::Id::new(("workspace_mask", mask));
    let tex = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)).or_else(|| {
        let bytes = base64::engine::general_purpose::STANDARD.decode(mask).ok()?;
        let side = ICON_SIDE as usize;
        if bytes.len() != side * side {
            return None;
        }
        let pixels = bytes
            .iter()
            .map(|a| egui::Color32::from_white_alpha(*a))
            .collect();
        let image = egui::ColorImage {
            size: [side, side],
            source_size: egui::vec2(side as f32, side as f32),
            pixels,
        };
        let t = ctx.load_texture("workspace_mask", image, egui::TextureOptions::LINEAR);
        ctx.data_mut(|d| d.insert_temp(id, t.clone()));
        Some(t)
    });
    if let Some(t) = tex {
        p.image(
            t.id(),
            r,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            color,
        );
    }
}

/// Draw a workspace's picture: one of the stock ones, or the user's.
pub fn paint_workspace_icon(p: &egui::Painter, r: egui::Rect, icon: &str, mask: &str, color: egui::Color32) {
    if icon == "custom" {
        paint_mask(p, r, mask, color);
    } else {
        paint_icon(p, r, icon, color);
    }
}

fn color32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

// ----- the switch -----------------------------------------------------------

/// The form for a new workspace or a workspace's look.
#[derive(Debug, Clone, PartialEq)]
struct LookForm {
    /// The workspace changed (`None`: a new one).
    id: Option<String>,
    name: String,
    color: [u8; 3],
    icon: String,
    mask: String,
    /// What the last try to make a picture of the user's found wrong.
    note: String,
}

fn form_id() -> egui::Id {
    egui::Id::new("workspace_look_form")
}

/// The switch at the right of the menu bar: one button per workspace, a
/// picture and the name, the one in use raised and underlined in its
/// colour; then "+" to save the panels as a new workspace.
pub fn switch(app: &mut App, ui: &mut egui::Ui) {
    use crate::i18n::tr;
    use crate::theme::Tokens;
    use crate::ui::tips::RichTip as _;
    let now = app.workspace_id();
    let frame = egui::Frame::new()
        .fill(Tokens::field())
        .corner_radius(7.0)
        .inner_margin(egui::Margin::same(2));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // Right to left: "+" first, then the last workspace first.
            let (pr, presp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
            if presp.hovered() {
                ui.painter().rect_filled(pr, 5.0, Tokens::tool_hover());
            }
            let col = if presp.hovered() { Tokens::text() } else { Tokens::text_dim() };
            let c = pr.center();
            let st = egui::Stroke::new(1.4, col);
            ui.painter().line_segment([c + egui::vec2(-4.5, 0.0), c + egui::vec2(4.5, 0.0)], st);
            ui.painter().line_segment([c + egui::vec2(0.0, -4.5), c + egui::vec2(0.0, 4.5)], st);
            if presp
                .rich_tip(crate::ui::tips::pair("persona.save_new", "persona.save_new_desc"))
                .clicked()
            {
                let n = app.settings.arcano_workspaces.len() + 1;
                let form = LookForm {
                    id: None,
                    name: crate::i18n::trf("persona.new_name", &[("n", &n.to_string())]),
                    color: COLORS[(n + 1) % COLORS.len()],
                    icon: ICONS[(n + 5) % (ICONS.len() - 1)].to_string(),
                    mask: String::new(),
                    note: String::new(),
                };
                ui.ctx().data_mut(|d| d.insert_temp(form_id(), Some(form)));
            }
            for w in app.workspaces().iter().rev() {
                let on = w.id == now;
                let galley = ui.painter().layout_no_wrap(
                    w.name.clone(),
                    egui::FontId::proportional(12.0),
                    Tokens::text(),
                );
                let icon_w = if w.icon == "none" || (w.icon == "custom" && w.mask.is_empty()) { 0.0 } else { 16.0 };
                let size = egui::vec2(galley.size().x + 20.0 + icon_w, 20.0);
                let (r, resp) = ui.allocate_exact_size(size, egui::Sense::click());
                if on {
                    ui.painter().rect_filled(r, 5.0, Tokens::panel());
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 5.0, Tokens::tool_hover());
                }
                let colour = color32(w.color);
                if on {
                    ui.painter().hline(
                        r.shrink2(egui::vec2(8.0, 0.0)).x_range(),
                        r.bottom() - 1.5,
                        egui::Stroke::new(2.0, colour),
                    );
                }
                let mut x = r.left() + 9.0;
                if icon_w > 0.0 {
                    let ir = egui::Rect::from_center_size(egui::pos2(x + 6.0, r.center().y), egui::vec2(13.0, 13.0));
                    paint_workspace_icon(ui.painter(), ir, &w.icon, &w.mask, if on { colour } else { Tokens::text_dim() });
                    x += icon_w;
                }
                ui.painter().galley(
                    egui::pos2(x, r.center().y - galley.size().y / 2.0),
                    galley,
                    if on { Tokens::text() } else { Tokens::text_dim() },
                );
                let tip = if w.builtin {
                    let key = if w.id == "page" { "persona.page" } else { "persona.drawing" };
                    format!("{}\n{}\n{}", w.name, tr(&format!("{key}_desc")), tr("persona.right_click"))
                } else {
                    format!("{}\n{}", w.name, tr("persona.right_click"))
                };
                let resp = resp.rich_tip(tip);
                if resp.clicked() {
                    app.switch_workspace(&w.id);
                }
                let w = w.clone();
                egui::Popup::context_menu(&resp)
                    .id(egui::Id::new(("workspace_menu", &w.id)))
                    .style(crate::ui::menus::menu_popup_style)
                    .show(|ui| {
                        crate::ui::menus::body(ui, |ui| {
                            if crate::ui::menus::item(ui, "persona.edit_look", "", true) {
                                let form = LookForm {
                                    id: Some(w.id.clone()),
                                    name: w.name.clone(),
                                    color: w.color,
                                    icon: w.icon.clone(),
                                    mask: w.mask.clone(),
                                    note: String::new(),
                                };
                                ui.ctx().data_mut(|d| d.insert_temp(form_id(), Some(form)));
                            }
                            if !w.builtin && crate::ui::menus::item(ui, "persona.resave", "", w.id == now) {
                                app.resave_workspace(&w.id);
                            }
                            if crate::ui::menus::item(ui, "persona.reset", "", w.id == now) {
                                app.reset_persona();
                            }
                            if !w.builtin {
                                crate::ui::menus::sep(ui);
                                if crate::ui::menus::item(ui, "persona.delete", "", true) {
                                    app.delete_workspace(&w.id);
                                }
                            }
                        })
                    });
            }
        });
    });
    look_window(app, ui.ctx());
}

/// The window that names a workspace and picks its colour and picture.
fn look_window(app: &mut App, ctx: &egui::Context) {
    use crate::i18n::tr;
    use crate::theme::Tokens;
    use crate::ui::tips::RichTip as _;
    let Some(mut form) = ctx.data(|d| d.get_temp::<Option<LookForm>>(form_id())).flatten() else {
        return;
    };
    let mut open = true;
    let mut done = None;
    let title = if form.id.is_none() { tr("persona.save_new") } else { tr("persona.edit_look") };
    egui::Window::new(title)
        .id(egui::Id::new("workspace_look_window"))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .default_pos(ctx.content_rect().right_top() + egui::vec2(-420.0, 40.0))
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.horizontal(|ui| {
                ui.label(tr("persona.name"));
                ui.add(egui::TextEdit::singleline(&mut form.name).desired_width(260.0).char_limit(28));
            });
            ui.add_space(6.0);
            ui.label(tr("persona.color"));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for c in COLORS {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                    ui.painter().rect_filled(r.shrink(2.0), 5.0, color32(c));
                    if form.color == c {
                        ui.painter().rect_stroke(r, 6.0, egui::Stroke::new(2.0, Tokens::text()), egui::StrokeKind::Inside);
                    }
                    let name = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
                    if resp.rich_tip(name).clicked() {
                        form.color = c;
                    }
                }
                let mut custom = tracedraw_core::Color::rgb8(form.color[0], form.color[1], form.color[2]);
                if crate::ui::color_picker::color_button(ui, &mut custom) {
                    form.color = custom.to_rgb8();
                }
            });
            ui.add_space(6.0);
            ui.label(tr("persona.icon"));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for icon in ICONS {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                    let on = form.icon == icon;
                    if on {
                        ui.painter().rect_filled(r, 5.0, Tokens::tool_active());
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 5.0, Tokens::tool_hover());
                    }
                    if icon == "none" {
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "\u{2205}", egui::FontId::proportional(13.0), Tokens::text_dim());
                    } else {
                        paint_icon(ui.painter(), r.shrink(6.0), icon, color32(form.color));
                    }
                    if resp.rich_tip(tr(&format!("persona.icon_{icon}"))).clicked() {
                        form.icon = icon.to_string();
                    }
                }
            });
            // A picture of the user's: the selected objects, or a file.
            ui.horizontal_wrapped(|ui| {
                let has = !form.mask.is_empty();
                let (r, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                if form.icon == "custom" && has {
                    ui.painter().rect_filled(r, 5.0, Tokens::tool_active());
                }
                if has {
                    paint_mask(ui.painter(), r.shrink(5.0), &form.mask, color32(form.color));
                    if resp.rich_tip(tr("persona.icon_custom")).clicked() {
                        form.icon = "custom".into();
                    }
                }
                if ui
                    .button(tr("persona.icon_from_selection"))
                    .rich_tip(crate::ui::tips::pair("persona.icon_from_selection", "persona.icon_from_selection_desc"))
                    .clicked()
                {
                    match mask_from_selection(app) {
                        Some(m) => {
                            form.mask = m;
                            form.icon = "custom".into();
                            form.note.clear();
                        }
                        None => form.note = tr("persona.icon_no_selection"),
                    }
                }
                if ui
                    .button(tr("persona.icon_import"))
                    .rich_tip(crate::ui::tips::pair("persona.icon_import", "persona.icon_import_desc"))
                    .clicked()
                {
                    let picked = std::rc::Rc::new(std::cell::RefCell::new(None));
                    let into = picked.clone();
                    crate::files::Dialog::new()
                        .add_filter(tr("persona.icon_files"), &["svg", "png", "jpg", "jpeg", "webp", "bmp", "gif"])
                        .pick_file(app, move |_, path| {
                            *into.borrow_mut() = crate::files::read(&path).ok();
                        });
                    let bytes = picked.borrow_mut().take();
                    if let Some(bytes) = bytes {
                        match mask_from_file(&bytes) {
                            Some(m) => {
                                form.mask = m;
                                form.icon = "custom".into();
                                form.note.clear();
                            }
                            None => form.note = tr("persona.icon_bad_file"),
                        }
                    }
                }
            });
            if !form.note.is_empty() {
                ui.label(egui::RichText::new(&form.note).small().color(Tokens::text_dim()));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.ok")).clicked() {
                    done = Some(true);
                }
                if ui.button(tr("dialog.cancel")).clicked() {
                    done = Some(false);
                }
            });
        });
    if !open || done == Some(false) {
        ctx.data_mut(|d| d.insert_temp::<Option<LookForm>>(form_id(), None));
        return;
    }
    if done == Some(true) {
        match &form.id {
            None => {
                app.save_workspace_with(&form.name, form.color, &form.icon, &form.mask);
            }
            Some(id) => app.set_workspace_look_with(id, &form.name, form.color, &form.icon, &form.mask),
        }
        ctx.data_mut(|d| d.insert_temp::<Option<LookForm>>(form_id(), None));
        return;
    }
    ctx.data_mut(|d| d.insert_temp(form_id(), Some(form)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_workspace_keeps_its_panels() {
        let mut app = App::headless();
        assert_eq!(app.persona(), Persona::Drawing);
        app.open_docker(DockerTab::Color);
        app.switch_persona(Persona::Page);
        assert_eq!(app.persona(), Persona::Page);
        // The page's panels, in their groups.
        for t in [
            DockerTab::Pages,
            DockerTab::Character,
            DockerTab::Paragraph,
            DockerTab::TextStyles,
            DockerTab::HousePress,
        ] {
            assert!(app.docker_open(t), "{t:?}");
        }
        assert!(!app.docker_open(DockerTab::Color));
        // Back: the drawing's panels as they were left.
        app.switch_persona(Persona::Drawing);
        assert!(app.docker_open(DockerTab::Color));
        assert!(!app.docker_open(DockerTab::Character));
        // A change kept for the next time.
        app.switch_persona(Persona::Page);
        app.open_docker(DockerTab::Glyphs);
        app.switch_persona(Persona::Drawing);
        app.switch_persona(Persona::Page);
        assert!(app.docker_open(DockerTab::Glyphs));
        // Reset: the preset again.
        app.reset_persona();
        assert!(!app.docker_open(DockerTab::Glyphs));
    }

    /// A workspace of the user's keeps the panels it was saved with, has
    /// its own colour (the accent while in use), can be changed and
    /// deleted; Page is orange.
    #[test]
    fn the_users_workspaces_save_panels_with_a_colour_and_a_picture() {
        let mut app = App::headless();
        assert_eq!(app.workspace_color(), DRAWING_COLOR);
        app.switch_persona(Persona::Page);
        assert_eq!(app.workspace_color(), PAGE_COLOR);
        app.open_docker(DockerTab::Glyphs);
        let blue = COLORS[2];
        let id = app.save_workspace("Capas", blue, "book");
        assert_eq!(app.workspace_id(), id);
        assert_eq!(app.workspace_color(), blue);
        assert_eq!(app.workspaces().len(), 3);
        assert!(app.docker_open(DockerTab::Glyphs));
        // Elsewhere and back: the panels it was saved with.
        app.switch_persona(Persona::Drawing);
        assert!(!app.docker_open(DockerTab::Glyphs));
        app.switch_workspace(&id);
        assert!(app.docker_open(DockerTab::Glyphs));
        // Its panels changed, then Restore: as saved.
        app.close_docker(DockerTab::Glyphs);
        app.reset_persona();
        assert!(app.docker_open(DockerTab::Glyphs));
        // A new look; a built-in one can take one too.
        app.set_workspace_look(&id, "Capas de livro", COLORS[4], "star");
        let w = app.workspaces().into_iter().find(|w| w.id == id).unwrap();
        assert_eq!((w.name.as_str(), w.color, w.icon.as_str()), ("Capas de livro", COLORS[4], "star"));
        app.set_workspace_look("page", "", COLORS[3], "type");
        let page = app.workspaces().into_iter().find(|w| w.id == "page").unwrap();
        assert_eq!(page.color, COLORS[3]);
        assert_eq!(page.name, crate::i18n::tr("persona.page"));
        // Deleted while in use: Drawing comes back.
        app.delete_workspace(&id);
        assert_eq!(app.workspace_id(), "drawing");
        assert_eq!(app.workspaces().len(), 2);
        // Built-in ones are never deleted.
        app.delete_workspace("page");
        assert_eq!(app.workspaces().len(), 2);
    }

    /// A picture of the user's is the objects' shape fitted in a square,
    /// from the selection or from a file; nothing to draw is no picture.
    #[test]
    fn a_picture_of_the_users_comes_from_objects_or_files() {
        use base64::Engine as _;
        // A 10 x 4 opaque bar in a 20 x 20 picture: wide, centred.
        let mut rgba = vec![0u8; 20 * 20 * 4];
        for y in 8..12 {
            for x in 5..15 {
                rgba[(y * 20 + x) * 4 + 3] = 255;
            }
        }
        let m = mask_from_rgba(&rgba, 20, 20).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(&m).unwrap();
        let side = ICON_SIDE as usize;
        assert_eq!(bytes.len(), side * side);
        assert!(bytes[side * (side / 2) + side / 2] > 200, "the middle shows");
        assert!(bytes[side * 4 + side / 2] < 10, "above the bar is clear");
        assert!(mask_from_rgba(&vec![0u8; 16 * 4], 4, 4).is_none());
        // From a file: an SVG square.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect x="5" y="5" width="30" height="30" fill="black"/></svg>"#;
        assert!(mask_from_file(svg.as_bytes()).is_some());
        assert!(mask_from_file(b"not a picture").is_none());
        // From the selection.
        let mut app = App::headless();
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 40.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        app.select(vec![id]);
        let m = mask_from_selection(&app).unwrap();
        let wid = app.save_workspace_with("Meu", COLORS[3], "custom", &m);
        let w = app.workspaces().into_iter().find(|w| w.id == wid).unwrap();
        assert_eq!((w.icon.as_str(), w.mask.is_empty()), ("custom", false));
    }

    /// The accent turns with the workspace's colour: the green itself
    /// becomes the colour, the pale green tints pale tints of it.
    #[test]
    fn the_accent_follows_the_workspace_colour() {
        let _g = crate::theme::button_sizes_lock();
        crate::theme::set_accent(PAGE_COLOR);
        let a = crate::theme::Tokens::accent();
        assert_eq!([a.r(), a.g(), a.b()], PAGE_COLOR);
        let tint = crate::theme::rehue(egui::Color32::from_rgb(0xD5, 0xF2, 0xDF));
        assert!(tint.r() > tint.b() && tint.r() > 200, "{tint:?}");
        crate::theme::set_accent(crate::theme::GREEN);
        assert_eq!(crate::theme::Tokens::accent(), crate::theme::Tokens::ACCENT);
    }
}
