//! Keeps the rasterized page as an egui texture, re-rendering only when the
//! document, view, canvas size or drag preview changed.

use crate::view::View;
use egui::{ColorImage, Context, Rect, TextureHandle, TextureId, TextureOptions};
use tracedraw_core::{geometry::Affine, Document, PageId, ShapeId};
use tracedraw_render::{render_page, Preview, RenderOptions, ViewTransform};

#[derive(Default)]
pub struct Raster {
    texture: Option<TextureHandle>,
    key: Option<Key>,
}

#[derive(PartialEq, Clone, Debug)]
struct Key {
    revision: u64,
    page: PageId,
    zoom: f32,
    /// The page origin relative to the canvas: what the render uses. A
    /// canvas that moves (a bar docked elsewhere) without changing size
    /// still needs a new render when the view stays put.
    offset: egui::Vec2,
    size: (u32, u32),
    preview: Option<(Vec<ShapeId>, [f64; 6])>,
    wireframe: bool,
    simulate_overprints: bool,
    complex_effects: bool,
}

impl Raster {
    /// Drop the cached texture so the next frame re-renders.
    pub fn invalidate(&mut self) {
        self.key = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn texture(
        &mut self,
        ctx: &Context,
        doc: &Document,
        page: PageId,
        view: &View,
        rect: Rect,
        preview: Option<(Vec<ShapeId>, Affine)>,
        revision: u64,
        wireframe: bool,
        simulate_overprints: bool,
        complex_effects: bool,
    ) -> Option<TextureId> {
        let ppp = ctx.pixels_per_point();
        let w = (rect.width() * ppp).round().max(1.0) as u32;
        let h = (rect.height() * ppp).round().max(1.0) as u32;
        let key = Key {
            revision,
            page,
            zoom: view.zoom,
            offset: view.origin - rect.min,
            size: (w, h),
            preview: preview.as_ref().map(|(s, t)| (s.clone(), t.as_coeffs())),
            wireframe,
            simulate_overprints,
            complex_effects,
        };
        if self.key.as_ref() == Some(&key) {
            if let Some(t) = &self.texture {
                return Some(t.id());
            }
        }
        let vt = ViewTransform {
            zoom: (view.zoom * ppp) as f64,
            origin_x: ((view.origin.x - rect.min.x) * ppp) as f64,
            origin_y: ((view.origin.y - rect.min.y) * ppp) as f64,
        };
        let opts = RenderOptions {
            width: w,
            height: h,
            view: vt,
            preview: preview.map(|(shapes, transform)| Preview { shapes, transform }),
            wireframe,
            simulate_overprints,
            complex_effects,
            anti_alias: true,
        };
        let pixmap = render_page(doc, page, &opts)?;
        let pixels: Vec<egui::Color32> = pixmap
            .pixels()
            .iter()
            .map(|p| {
                egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha())
            })
            .collect();
        let image = ColorImage {
            size: [w as usize, h as usize],
            source_size: egui::vec2(w as f32, h as f32),
            pixels,
        };
        match &mut self.texture {
            Some(t) => t.set(image, TextureOptions::LINEAR),
            None => self.texture = Some(ctx.load_texture("page", image, TextureOptions::LINEAR)),
        }
        self.key = Some(key);
        self.texture.as_ref().map(|t| t.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Size;

    #[test]
    fn a_canvas_that_moves_renders_again() {
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let page = doc.pages[0].id;
        let view = View::default();
        let mut raster = Raster::default();
        let at = |x: f32, y: f32| Rect::from_min_size(egui::pos2(x, y), egui::vec2(320.0, 200.0));
        let draw = |raster: &mut Raster, rect: Rect| {
            raster.texture(&ctx, &doc, page, &view, rect, None, 1, false, false, false)
        };
        assert!(draw(&mut raster, at(40.0, 160.0)).is_some());
        let first = raster.key.clone();
        // Same canvas, same view: the cached texture.
        assert!(draw(&mut raster, at(40.0, 160.0)).is_some());
        assert_eq!(raster.key, first);
        // The canvas moves up by a bar's height and keeps its size: the
        // page sits elsewhere inside it, so the key changes.
        assert!(draw(&mut raster, at(40.0, 107.0)).is_some());
        assert_ne!(raster.key, first);
        assert_eq!(
            raster.key.as_ref().map(|k| k.offset),
            Some(view.origin - egui::pos2(40.0, 107.0))
        );
    }
}
