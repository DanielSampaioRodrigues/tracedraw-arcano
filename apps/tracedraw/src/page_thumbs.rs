//! Arcano edition: the miniatures of the Pages panel, of each page and of
//! each master page's left-hand and right-hand page.
//!
//! A miniature is drawn again only when what it shows changed: when the
//! drawing's revision moved on, the layers drawn on that page (its own and
//! its masters', page number fields filled in) are compared with the copy
//! the picture was drawn from. At most a few milliseconds of new pictures
//! are drawn per frame; the others keep their old picture (or none) until
//! the next frames.

use std::cell::RefCell;
use std::collections::HashMap;
use tracedraw_core::document::{Layer, PageMaster};
use tracedraw_core::{Document, PageId};

/// Time for new pictures in one frame, milliseconds (one is always drawn).
const BUDGET_MS: f64 = 8.0;
/// Pictures kept before the least recently shown are dropped.
const MAX_ENTRIES: usize = 600;

/// What a miniature shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Of {
    /// The page, at its index (number fields depend on it).
    Page(PageId, usize),
    /// A master page's left-hand (true) or right-hand page.
    Master(u32, bool),
}

struct Entry {
    revision: u64,
    seen: Vec<Layer>,
    size: (f64, f64),
    px: u32,
    texture: Option<egui::TextureHandle>,
    used: u64,
}

#[derive(Default)]
struct Cache {
    entries: HashMap<(u64, Of), Entry>,
    frame: u64,
    drawn: usize,
    started: Option<web_time::Instant>,
    waiting: bool,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// Start a frame of the panel: the drawing budget starts again. Returns
/// whether pictures waited in the last frame (the panel then asks for
/// another frame).
pub fn begin_frame() -> bool {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let waited = c.waiting;
        c.frame += 1;
        c.drawn = 0;
        c.started = None;
        c.waiting = false;
        if c.entries.len() > MAX_ENTRIES {
            let frame = c.frame;
            let mut order: Vec<((u64, Of), u64)> =
                c.entries.iter().map(|(k, e)| (*k, e.used)).collect();
            order.sort_by_key(|(_, u)| *u);
            let excess = c.entries.len() - MAX_ENTRIES;
            for (k, used) in order.into_iter().take(excess) {
                if used + 1 < frame {
                    c.entries.remove(&k);
                }
            }
        }
        waited
    })
}

/// The layers a miniature shows, and the page size, from `doc`.
fn content(doc: &Document, of: Of) -> Option<(Vec<Layer>, (f64, f64))> {
    match of {
        Of::Page(id, _) => {
            let p = doc.page(id).ok()?;
            let layers = doc
                .layers_drawn_for_page(id)
                .ok()?
                .into_iter()
                .map(|l| l.into_owned())
                .collect();
            Some((layers, (p.size.width, p.size.height)))
        }
        Of::Master(m, _) => {
            let chain = doc.master_chain(m);
            let p = doc.pages.first()?;
            let layers = doc
                .master
                .iter()
                .filter(|l| chain.contains(&l.master_page))
                .cloned()
                .collect();
            Some((layers, (p.size.width, p.size.height)))
        }
    }
}

/// A drawing of master page `m` alone: two pages of the first page's
/// size with only that master, the right-hand page first; returns it and
/// the page asked for.
fn master_doc(doc: &Document, m: u32, left: bool) -> Option<(Document, PageId)> {
    let mut d = doc.clone();
    let mut first = d.pages.first()?.clone();
    for l in &mut first.layers {
        l.shapes.clear();
    }
    first.background = None;
    let mut right = first.clone();
    right.id = PageId(u64::MAX - 1);
    let mut left_page = first;
    left_page.id = PageId(u64::MAX - 2);
    let want = if left { left_page.id } else { right.id };
    d.pages = vec![right, left_page];
    d.sections.clear();
    d.page_masters = d
        .pages
        .iter()
        .map(|p| PageMaster {
            page: p.id,
            master: m,
        })
        .collect();
    Some((d, want))
}

/// The picture for `of` in drawing `instance` at `revision`, `px` pixels
/// tall, or the older one (or none) while it waits its turn.
pub fn get(
    ctx: &egui::Context,
    doc: &Document,
    (instance, revision): (u64, u64),
    of: Of,
    px: u32,
) -> Option<egui::TextureId> {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let frame = c.frame;
        let key = (instance, of);
        if let Some(e) = c.entries.get_mut(&key) {
            e.used = frame;
            if e.px == px && e.revision == revision {
                return e.texture.as_ref().map(|t| t.id());
            }
        }
        let (layers, size) = content(doc, of)?;
        if let Some(e) = c.entries.get_mut(&key) {
            if e.px == px && e.size == size && e.seen == layers {
                e.revision = revision;
                return e.texture.as_ref().map(|t| t.id());
            }
        }
        let over = c.drawn > 0
            && c
                .started
                .is_some_and(|t| t.elapsed().as_secs_f64() * 1000.0 > BUDGET_MS);
        if over {
            c.waiting = true;
            return c
                .entries
                .get(&key)
                .and_then(|e| e.texture.as_ref().map(|t| t.id()));
        }
        if c.drawn == 0 {
            c.started = Some(web_time::Instant::now());
        }
        c.drawn += 1;
        let dpi = (px as f64 / (size.1.max(1.0) / 25.4)).max(2.0);
        let pixmap = match of {
            Of::Page(id, _) => tracedraw_render::render_page_image(doc, id, dpi),
            Of::Master(m, left) => master_doc(doc, m, left)
                .and_then(|(d, p)| tracedraw_render::render_page_image(&d, p, dpi)),
        };
        let texture = pixmap.map(|pm| {
            let image = egui::ColorImage::from_rgba_premultiplied(
                [pm.width() as usize, pm.height() as usize],
                pm.data(),
            );
            let name = match of {
                Of::Page(id, i) => format!("page-thumb:{instance}:{}:{i}", id.raw()),
                Of::Master(m, l) => format!("master-thumb:{instance}:{m}:{l}"),
            };
            ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
        });
        let id = texture.as_ref().map(|t| t.id());
        c.entries.insert(
            key,
            Entry {
                revision,
                seen: layers,
                size,
                px,
                texture,
                used: frame,
            },
        );
        id
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::Command;

    #[test]
    fn a_miniature_is_drawn_again_only_when_its_page_changed() {
        let ctx = egui::Context::default();
        let mut app = crate::app::App::headless();
        app.add_page();
        let pages: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        let key = |app: &crate::app::App| (app.engine.instance(), app.engine.revision());
        begin_frame();
        let a = get(&ctx, app.doc(), key(&app), Of::Page(pages[0], 0), 40);
        let b = get(&ctx, app.doc(), key(&app), Of::Page(pages[1], 1), 40);
        assert!(a.is_some() && b.is_some());
        // Something drawn on page 2: page 1 keeps its picture.
        app.new_shape(tracedraw_core::ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .unwrap();
        begin_frame();
        assert_eq!(get(&ctx, app.doc(), key(&app), Of::Page(pages[0], 0), 40), a);
        let drawn = CACHE.with(|c| c.borrow().drawn);
        assert_eq!(drawn, 0);
        let _ = get(&ctx, app.doc(), key(&app), Of::Page(pages[1], 1), 40);
        assert_eq!(CACHE.with(|c| c.borrow().drawn), 1);
        // A master page draws alone.
        app.run(Command::NewMasterPage {
            prefix: "B".into(),
            name: String::new(),
            based_on: None,
            layer_name: "B".into(),
        });
        begin_frame();
        assert!(get(&ctx, app.doc(), key(&app), Of::Master(1, false), 40).is_some());
    }
}
