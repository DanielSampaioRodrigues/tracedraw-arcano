//! Several open drawings, one tab each.
//!
//! The active drawing's state lives in the `App` fields that the rest of
//! the editor already uses (`engine`, `page`, `view`, `selection`, `file`
//! ...). Every other open drawing waits in a [`DocSlot`]. Switching tabs
//! swaps the fields with the slot, so no other code needs to know how
//! many drawings are open. The slot at `active_doc` is a placeholder
//! while that drawing is active.
//!
//! With no drawing open (only the Welcome Screen) `docs` is empty and the
//! fields hold an empty placeholder document that is never shown.

use crate::app::{App, Drag, PageNumberSettings};
use crate::view::View;
use std::path::PathBuf;
use tracedraw_core::{Document, Engine, PageId, ShapeId};

/// Everything that belongs to one open drawing.
#[derive(Default)]
pub struct DocSlot {
    pub engine: Engine,
    pub page: Option<PageId>,
    pub view: View,
    pub selection: Vec<ShapeId>,
    pub file: Option<PathBuf>,
    pub cdr_version: Option<u16>,
    pub fit_pending: bool,
    pub page_numbers: PageNumberSettings,
    pub missing_fonts: Vec<String>,
    /// Drawing units of this drawing (the property bar's Units).
    pub units: crate::app::Units,
    /// Centres of rotation moved in this drawing.
    pub pivots: crate::app::Pivots,
    /// The layer made active in the Objects docker.
    pub picked_layer: Option<tracedraw_core::LayerId>,
    /// The drawing's rendered page, kept while another drawing is active
    /// (its window shows it when windows are tiled or cascaded).
    pub raster: crate::raster::Raster,
    /// Where the drawing's canvas began on screen when it was last drawn
    /// (see `App::anchor_view`).
    pub view_anchor: Option<egui::Pos2>,
}

impl App {
    /// True when at least one drawing is open (not only the Welcome Screen).
    pub fn has_document(&self) -> bool {
        !self.docs.is_empty()
    }

    /// Swap the active-drawing fields with slot `i` (twice puts everything
    /// back, which is how a window that is not the active one is drawn).
    pub(crate) fn swap_with_slot(&mut self, i: usize) {
        let Some(slot) = self.docs.get_mut(i) else {
            return;
        };
        std::mem::swap(&mut self.engine, &mut slot.engine);
        let page = slot.page.unwrap_or(self.page);
        slot.page = Some(self.page);
        self.page = page;
        std::mem::swap(&mut self.view, &mut slot.view);
        std::mem::swap(&mut self.selection, &mut slot.selection);
        std::mem::swap(&mut self.file, &mut slot.file);
        std::mem::swap(&mut self.cdr_version, &mut slot.cdr_version);
        std::mem::swap(&mut self.fit_pending, &mut slot.fit_pending);
        std::mem::swap(&mut self.page_numbers, &mut slot.page_numbers);
        std::mem::swap(&mut self.missing_fonts, &mut slot.missing_fonts);
        std::mem::swap(&mut self.units, &mut slot.units);
        std::mem::swap(&mut self.pivots, &mut slot.pivots);
        std::mem::swap(&mut self.picked_layer, &mut slot.picked_layer);
        std::mem::swap(self.raster.get_mut(), &mut slot.raster);
        // Measured in the other drawing.
        *self.visual_cache.get_mut() = (u64::MAX, Default::default());
        std::mem::swap(&mut self.view_anchor, &mut slot.view_anchor);
        // A page id from another drawing means nothing here.
        if self.engine.document().page(self.page).is_err() {
            if let Some(p) = self.engine.document().pages.first() {
                self.page = p.id;
            }
        }
    }

    /// Drop the in-progress interactions of the drawing being left: they
    /// refer to its objects.
    fn end_transient_editing(&mut self) {
        // Time counted for the drawing being left belongs to it.
        self.flush_tracked_time();
        if self.clip_frame_edit.is_some() {
            self.finish_clip_frame_edit();
        }
        self.drag = Drag::None;
        self.curve = None;
        self.text_edit = None;
        self.table_edit = None;
        self.three_point_base = None;
        self.dimension_points.clear();
        self.node_selection.clear();
        self.selected_guides.clear();
        self.guide_rotate = None;
        self.rotate_mode = false;
        self.pending_clip_frame = false;
        self.pending_order = None;
        self.pending_copy_properties = false;
        self.pending_copy_effect = None;
        self.pending_copy_envelope = false;
        self.envelope_drag = None;
        self.pending_clone_effect = None;
        self.pending_blend_path = false;
        self.anchor_sel = None;
        self.effect_node_drag = None;
        self.selected_effect_node = None;
        self.lens_synced_to = None;
        self.extrude_synced_to = None;
        self.context_menu = None;
        self.navigator_tex = None;
        self.last_repeatable = None;
        self.flyout_open = None;
        self.raster.borrow_mut().invalidate();
    }

    /// Open `doc` in a new tab after the others and make it active.
    pub fn open_document(&mut self, doc: Document, file: Option<PathBuf>) {
        self.end_transient_editing();
        // A new drawing starts in the units in use.
        let units = self.units;
        if self.has_document() {
            // Park the active drawing in its slot.
            self.swap_with_slot(self.active_doc);
        }
        self.page = doc.pages.first().map(|p| p.id).unwrap_or(self.page);
        self.add_carried_fonts(&doc.fonts);
        self.engine = Engine::new(doc);
        self.engine
            .set_max_history(self.settings.undo_levels.max(1));
        self.selection.clear();
        self.file = file;
        self.cdr_version = None;
        self.view = View::default();
        self.fit_pending = true;
        self.page_numbers = PageNumberSettings::default();
        self.missing_fonts.clear();
        self.units = units;
        self.pivots = Default::default();
        self.picked_layer = None;
        *self.raster.get_mut() = Default::default();
        self.view_anchor = None;
        self.docs.push(DocSlot::default());
        self.active_doc = self.docs.len() - 1;
        self.show_welcome = false;
    }

    /// Fonts a drawing carries serve its text (and the font lists) for the
    /// rest of the session.
    pub fn add_carried_fonts(&mut self, fonts: &[tracedraw_core::DocumentFont]) {
        if !fonts.is_empty() && !tracedraw_text::fonts().add_document_fonts(fonts).is_empty() {
            self.sync_fonts();
        }
    }

    /// Catch up with fonts added since the last frame (fetched by the
    /// browser build when text asked for them, or carried by a drawing):
    /// refresh the font lists and the missing-font list, and draw the
    /// page and the text measurements again, since text drawn with a
    /// stand-in may now have its own font.
    pub fn sync_fonts(&mut self) {
        let fonts = tracedraw_text::fonts();
        let generation = fonts.generation();
        if generation == self.font_generation {
            return;
        }
        self.font_generation = generation;
        self.font_families = fonts.available_families();
        self.raster.borrow_mut().invalidate();
        for slot in &mut self.docs {
            slot.raster.invalidate();
        }
        self.navigator_tex = None;
        *self.overflow_cache.borrow_mut() = Default::default();
        if !self.missing_fonts.is_empty() {
            let families = &self.font_families;
            self.missing_fonts.retain(|f| !families.contains(f));
        }
    }

    /// Bring tab `i` to the front.
    pub fn switch_document(&mut self, i: usize) {
        if i >= self.docs.len() {
            return;
        }
        self.show_welcome = false;
        if i == self.active_doc {
            return;
        }
        self.end_transient_editing();
        self.swap_with_slot(self.active_doc);
        self.swap_with_slot(i);
        self.active_doc = i;
    }

    /// Close the active drawing without asking (the caller saved or the
    /// user chose not to). The tab to its right becomes active, else the
    /// one to its left; with none left the Welcome Screen shows.
    pub fn close_active_document(&mut self) {
        if !self.has_document() {
            return;
        }
        self.end_transient_editing();
        let i = self.active_doc.min(self.docs.len() - 1);
        self.docs.remove(i);
        if self.docs.is_empty() {
            self.engine = Engine::default();
            self.page = self.engine.document().pages[0].id;
            self.selection.clear();
            self.file = None;
            self.cdr_version = None;
            self.view = View::default();
            self.fit_pending = true;
            self.page_numbers = PageNumberSettings::default();
            self.missing_fonts.clear();
            self.pivots = Default::default();
            self.picked_layer = None;
            *self.raster.get_mut() = Default::default();
            self.view_anchor = None;
            self.active_doc = 0;
            self.show_welcome = true;
            return;
        }
        let next = i.min(self.docs.len() - 1);
        // The fields still hold the closed drawing: swapping puts it in the
        // slot, where it is dropped.
        self.swap_with_slot(next);
        if let Some(slot) = self.docs.get_mut(next) {
            *slot = DocSlot::default();
        }
        self.active_doc = next;
    }

    /// Tab label of drawing `i`: the file or document name, with an
    /// asterisk while it has unsaved changes.
    pub fn document_tab_title(&self, i: usize) -> String {
        if i == self.active_doc {
            return self.document_title();
        }
        let Some(slot) = self.docs.get(i) else {
            return String::new();
        };
        let name = slot
            .file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| slot.engine.document().title.clone());
        if slot.engine.is_dirty() {
            format!("{name}*")
        } else {
            name
        }
    }

    /// Indices of the open drawings with unsaved changes.
    pub fn dirty_documents(&self) -> Vec<usize> {
        (0..self.docs.len())
            .filter(|&i| {
                if i == self.active_doc {
                    self.engine.is_dirty()
                } else {
                    self.docs[i].engine.is_dirty()
                }
            })
            .collect()
    }

    /// Window > Close All: close every drawing, stopping at each one with
    /// unsaved changes to ask (the question continues the run).
    pub fn close_all_documents(&mut self) {
        self.closing_all = true;
        while self.has_document() {
            if self.engine.is_dirty() {
                self.show_welcome = false;
                self.dialog = crate::ui::dialogs::Dialog::ConfirmClose;
                return;
            }
            self.close_active_document();
        }
        self.closing_all = false;
        if self.quit_after_closing {
            self.quit_after_closing = false;
            self.quit_now = true;
        }
    }

    /// File > Exit and the window's close button: ask about every drawing
    /// with unsaved changes, then quit.
    pub fn request_exit(&mut self) {
        if self.dirty_documents().is_empty() {
            self.quit_now = true;
            return;
        }
        self.quit_after_closing = true;
        self.close_all_documents();
    }

    /// The Save / Don't Save / Cancel question was answered.
    pub fn after_close_question(&mut self, closed: bool) {
        if closed && self.closing_all {
            self.close_all_documents();
        } else if !closed {
            self.closing_all = false;
            self.quit_after_closing = false;
        }
    }

    /// The next free "Untitled-N" name of this session.
    pub fn next_untitled_name(&mut self) -> String {
        self.untitled_counter += 1;
        crate::i18n::trf(
            "doc.untitled_n",
            &[("n", &self.untitled_counter.to_string())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::document::ShapeKind;
    use tracedraw_core::geometry::{Rect, Size};

    fn add_rect(app: &mut App) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer")
    }

    #[test]
    fn each_drawing_keeps_its_objects_history_and_selection() {
        let mut app = App::headless();
        assert_eq!(app.docs.len(), 1);
        let a = add_rect(&mut app);
        app.select(vec![a]);
        app.open_document(Document::new("Second", Size::new(100.0, 50.0)), None);
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.active_doc, 1);
        assert!(app.selection.is_empty());
        assert_eq!(app.page_size().width, 100.0);
        add_rect(&mut app);
        add_rect(&mut app);
        app.switch_document(0);
        assert_eq!(app.selection, vec![a]);
        assert_eq!(app.page_size().width, 210.0);
        let count = |app: &App| {
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        assert_eq!(count(&app), 1);
        app.undo();
        assert_eq!(count(&app), 0);
        app.switch_document(1);
        assert_eq!(count(&app), 2);
        assert_eq!(app.document_tab_title(0), app.document_tab_title(0));
        assert!(app.document_tab_title(1).ends_with('*'));
    }

    #[test]
    fn closing_moves_to_a_neighbour_then_to_the_welcome_screen() {
        let mut app = App::headless();
        app.open_document(Document::new("B", Size::new(50.0, 50.0)), None);
        app.open_document(Document::new("C", Size::new(60.0, 60.0)), None);
        app.switch_document(1);
        app.close_active_document();
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.doc().title, "C");
        app.close_active_document();
        app.close_active_document();
        assert!(!app.has_document());
        assert!(app.show_welcome);
        // Closing again with nothing open is harmless.
        app.close_active_document();
        app.switch_document(3);
    }

    #[test]
    fn exit_asks_about_each_unsaved_drawing_then_quits() {
        let mut app = App::headless();
        add_rect(&mut app);
        app.open_document(Document::new("Clean", Size::new(50.0, 50.0)), None);
        app.open_document(Document::new("Dirty", Size::new(50.0, 50.0)), None);
        add_rect(&mut app);
        app.request_exit();
        // The clean drawings closed; the first unsaved one asks.
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ConfirmClose
        ));
        assert!(!app.quit_now);
        let asked = app.doc().title.clone();
        // Don't Save: on to the next unsaved drawing.
        app.dialog = crate::ui::dialogs::Dialog::None;
        app.close_active_document();
        app.after_close_question(true);
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ConfirmClose
        ));
        assert_ne!(app.doc().title, asked);
        // Cancel stops quitting.
        app.after_close_question(false);
        assert!(!app.quit_now && !app.closing_all);
        // Asked again, Don't Save on the last one quits.
        app.request_exit();
        app.close_active_document();
        app.after_close_question(true);
        assert!(app.quit_now);
        assert!(!app.has_document());
    }

    /// A drawing whose only text, inside a group, is set in `family`.
    fn drawing_with_text_in(family: &str) -> Document {
        let mut doc = Document::new(family, Size::new(100.0, 50.0));
        let text = tracedraw_core::Shape::new(
            ShapeId(1),
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("Hi", family, 12.0)],
                origin: tracedraw_core::geometry::Point::new(10.0, 10.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        doc.pages[0].layers[0]
            .shapes
            .push(tracedraw_core::Shape::new(
                ShapeId(2),
                ShapeKind::Group {
                    children: vec![text],
                },
            ));
        doc
    }

    #[test]
    fn a_drawing_brings_its_fonts_along() {
        let mut app = App::headless();
        let mut doc = drawing_with_text_in("Selawik");
        doc.fonts.push(tracedraw_core::DocumentFont::new(
            "Selawik",
            include_bytes!("../assets/fonts/Selawik-Regular.ttf").to_vec(),
        ));
        app.open_document(doc, None);
        app.check_missing_fonts();
        assert!(app.font_families.iter().any(|f| f == "Selawik"));
        assert!(app.missing_fonts.is_empty(), "{:?}", app.missing_fonts);
        // A font nobody brings is reported, inside a group too.
        app.open_document(drawing_with_text_in("Nowhere Grotesk"), None);
        app.check_missing_fonts();
        assert_eq!(app.missing_fonts, vec!["Nowhere Grotesk".to_string()]);
    }

    #[test]
    fn fonts_that_arrive_later_refresh_the_lists() {
        let mut app = App::headless();
        app.missing_fonts = vec!["Quicksand".into(), "Nowhere Grotesk".into()];
        // Nothing new: nothing changes.
        app.sync_fonts();
        assert_eq!(app.missing_fonts.len(), 2);
        // A font arrives, as the browser build's font folder delivers them.
        let quicksand = include_bytes!("../../../crates/text/testdata/Quicksand-VF.ttf");
        let _ = tracedraw_text::fonts().add_fonts(vec![quicksand.to_vec()]);
        app.sync_fonts();
        assert_eq!(app.font_generation, tracedraw_text::fonts().generation());
        assert!(app.font_families.iter().any(|f| f == "Quicksand"));
        assert_eq!(app.missing_fonts, vec!["Nowhere Grotesk".to_string()]);
    }

    #[test]
    fn untitled_names_count_up() {
        let mut app = App::headless();
        let a = app.next_untitled_name();
        let b = app.next_untitled_name();
        assert_ne!(a, b);
    }
}
