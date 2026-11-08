//! Arcano edition: the primary text frame of a master page. A master
//! with one ([`tracedraw_core::PrimaryFrame`]) gives every page made with
//! it a paragraph frame inside its margins, threaded on from the primary
//! frame of the page before (and into the one of the page after), so a
//! text put in the first frame runs through the pages and on into pages
//! added later. See docs/behavior/pages-panel.md.

use crate::app::App;
use tracedraw_core::{PageId, PrimaryFrame, ShapeId, ShapeKind, TextSpan};

/// The object data key that marks a page's primary frame.
pub const PRIMARY: &str = "arcano.primary";

fn is_primary(s: &tracedraw_core::Shape) -> bool {
    matches!(s.kind, ShapeKind::Text { frame: Some(_), .. })
        && s.data.iter().any(|(k, v)| k == PRIMARY && v == "1")
}

/// Columns and gutter on a frame just made, over the defaults' columns
/// that [`App::new_shape`] gives it.
pub(crate) fn keep_columns(s: &mut tracedraw_core::Shape, columns: u32, gutter: f64) {
    if let ShapeKind::Text { para, .. } = &mut s.kind {
        para.columns = columns.clamp(1, 20);
        para.gutter = if gutter.is_finite() { gutter.max(0.0) } else { 0.0 };
    }
}

impl App {
    /// The primary frame the master of `page` gives: its own, else the
    /// first of the masters it is based on that has one.
    pub fn primary_of_page(&self, page: PageId) -> Option<PrimaryFrame> {
        let doc = self.doc();
        let m = doc.master_of(page);
        doc.master_chain(m)
            .iter()
            .rev()
            .find_map(|id| doc.master_page(*id).and_then(|mp| mp.primary))
    }

    /// The primary frame on the page at index `idx`, if it has one.
    pub fn primary_frame_on(&self, idx: usize) -> Option<ShapeId> {
        let page = self.doc().pages.get(idx)?;
        page.layers
            .iter()
            .flat_map(|l| l.shapes.iter())
            .find(|s| is_primary(s))
            .map(|s| s.id)
    }

    /// A blank paragraph frame at `origin`, `size` big, with `columns`
    /// columns `gutter` mm apart, in the text settings in use.
    pub(crate) fn blank_frame_kind(
        &self,
        origin: tracedraw_core::Point,
        size: tracedraw_core::Size,
        columns: u32,
        gutter: f64,
    ) -> ShapeKind {
        let span = TextSpan {
            bold: self.text_bold,
            italic: self.text_italic,
            ..TextSpan::new("", self.text_font.clone(), self.text_size_pt)
        };
        let mut para = tracedraw_core::ParagraphStyle::default();
        para.hyphenate = self.text_hyphenation;
        para.columns = columns.clamp(1, 20);
        para.gutter = if gutter.is_finite() { gutter.max(0.0) } else { 0.0 };
        ShapeKind::Text {
            spans: vec![span],
            origin,
            frame: Some(size),
            align: self.text_align,
            para,
            on_path: None,
        }
    }

    /// Put the primary frame on `page` when its master has one and the
    /// page has none, threaded between the primary frames of the nearest
    /// pages before and after it; part of the caller's undo step. The
    /// frame made.
    pub fn add_primary_frame(&mut self, page: PageId) -> Option<ShapeId> {
        let primary = self.primary_of_page(page)?;
        let idx = self.doc().pages.iter().position(|p| p.id == page)?;
        if self.primary_frame_on(idx).is_some() {
            return None;
        }
        let size = self.doc().pages.get(idx)?.size;
        let facing = self.doc().metadata.facing_pages;
        let right_hand = !facing || idx % 2 == 0;
        let (origin, frame) = primary.rect_on(size, right_hand);
        let kind = self.blank_frame_kind(origin, frame, primary.columns, primary.gutter);
        let keep_page = self.page;
        let keep_layer = self.picked_layer;
        self.page = page;
        self.picked_layer = None;
        let made = self.new_shape_styled(kind, |s| {
            keep_columns(s, primary.columns, primary.gutter);
            s.data.push((PRIMARY.into(), "1".into()));
        });
        self.page = keep_page;
        self.picked_layer = keep_layer;
        let made = made?;
        let prev = (0..idx).rev().find_map(|i| self.primary_frame_on(i));
        let n = self.doc().pages.len();
        let next = (idx + 1..n).find_map(|i| self.primary_frame_on(i));
        self.thread_between(made, prev, next);
        Some(made)
    }

    /// A drawing just made with New Document's primary text frame:
    /// master A gets one in the house margins and every page its frame,
    /// threaded, as the drawing's starting state (nothing to undo).
    pub fn start_with_primary_frames(&mut self) {
        let h = crate::prelo_casa::house_prefs(self);
        let Some(mut a) = self.doc().master_page(0) else {
            return;
        };
        a.primary = Some(PrimaryFrame {
            margins: [h.top, h.bottom, h.inner, h.outer],
            columns: 1,
            gutter: 5.0,
        });
        self.run(tracedraw_core::Command::SetMasterPage { master: a });
        self.add_primary_frames_for_master(0);
        let first = self.primary_frame_on(0);
        let doc = self.doc().clone();
        self.engine.replace(doc);
        if let Some(f) = first {
            self.select(vec![f]);
        }
    }

    /// Primary frames on every page that uses master `master` (or a
    /// master based on it) and has none; the number made.
    pub fn add_primary_frames_for_master(&mut self, master: u32) -> usize {
        let pages: Vec<PageId> = self.doc().pages.iter().map(|p| p.id).collect();
        let mark = self.engine.begin_group();
        let mut made = 0;
        for p in pages {
            let m = self.doc().master_of(p);
            if self.doc().master_chain(m).contains(&master) && self.add_primary_frame(p).is_some() {
                made += 1;
            }
        }
        self.engine.end_group(mark, "Primary Text Frames");
        made
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::Command;

    fn with_primary(app: &mut App, margins: [f64; 4]) {
        let mut a = app.doc().master_page(0).unwrap();
        a.primary = Some(PrimaryFrame {
            margins,
            columns: 1,
            gutter: 5.0,
        });
        app.run(Command::SetMasterPage { master: a });
    }

    fn frame_rect(app: &App, id: ShapeId) -> (f64, f64, f64, f64) {
        match &app.doc().find_shape(id).unwrap().kind {
            ShapeKind::Text {
                origin,
                frame: Some(sz),
                ..
            } => (origin.x, origin.y, sz.width, sz.height),
            _ => panic!("not a frame"),
        }
    }

    #[test]
    fn pages_made_with_a_primary_master_get_threaded_frames() {
        let mut app = App::headless();
        app.set_facing_pages(true);
        with_primary(&mut app, [20.0, 25.0, 22.0, 18.0]);
        assert_eq!(app.add_primary_frames_for_master(0), 1);
        let first = app.primary_frame_on(0).unwrap();
        app.add_page();
        app.add_page();
        let second = app.primary_frame_on(1).unwrap();
        let third = app.primary_frame_on(2).unwrap();
        assert_eq!(app.chain_of(first), vec![first, second, third]);
        // Inner and outer swap on the left-hand page.
        let w = app.doc().pages[0].size.width;
        let (x0, y0, fw, _) = frame_rect(&app, first);
        assert!((x0 - 22.0).abs() < 1e-9 && (y0 - 25.0).abs() < 1e-9);
        assert!((fw - (w - 40.0)).abs() < 1e-9);
        let (x1, ..) = frame_rect(&app, second);
        assert!((x1 - 18.0).abs() < 1e-9);
        // A page inserted in the middle goes into the chain where it is.
        app.goto_page(0);
        app.insert_page(true);
        let middle = app.primary_frame_on(1).unwrap();
        assert_eq!(app.chain_of(first), vec![first, middle, second, third]);
        // One undo takes the page and its frame away, the chain whole.
        app.engine.undo().unwrap();
        assert_eq!(app.chain_of(first), vec![first, second, third]);
    }

    #[test]
    fn text_in_the_primary_frame_runs_on_into_new_pages() {
        let mut app = App::headless();
        with_primary(&mut app, [20.0, 25.0, 22.0, 18.0]);
        app.add_primary_frames_for_master(0);
        let first = app.primary_frame_on(0).unwrap();
        let words = "palavra ".repeat(900);
        let mut kind = app.doc().find_shape(first).unwrap().kind.clone();
        if let ShapeKind::Text { spans, .. } = &mut kind {
            spans[0].text = words;
        }
        app.run(Command::SetShapeKind { shape: first, kind });
        let added = app.autoflow(first);
        assert!(added > 0);
        // Every new page has its one primary frame, all in one chain.
        let n = app.doc().pages.len();
        let frames: Vec<ShapeId> = (0..n).filter_map(|i| app.primary_frame_on(i)).collect();
        assert_eq!(frames.len(), n);
        assert_eq!(app.chain_of(first), frames);
    }

    #[test]
    fn a_new_drawing_can_start_with_its_primary_frames() {
        let mut app = App::headless();
        let mut st = crate::new_document::NewDocState::from_settings(
            &crate::new_document::NewDocSettings::default(),
            "Livro".into(),
        );
        st.settings.pages = 3;
        st.settings.facing = true;
        st.settings.primary_frame = true;
        crate::new_document::create(&mut app, &st);
        assert!(app.doc().metadata.facing_pages);
        let frames: Vec<ShapeId> = (0..3).filter_map(|i| app.primary_frame_on(i)).collect();
        assert_eq!(frames.len(), 3);
        assert_eq!(app.chain_of(frames[0]), frames);
        assert!(!app.engine.is_dirty());
        assert!(app.engine.undo().is_err());
    }

    #[test]
    fn a_master_without_a_primary_frame_adds_none() {
        let mut app = App::headless();
        app.add_page();
        assert!(app.primary_frame_on(1).is_none());
        let shapes: usize = app.doc().pages[1].layers.iter().map(|l| l.shapes.len()).sum();
        assert_eq!(shapes, 0);
    }
}
