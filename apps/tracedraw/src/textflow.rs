//! Linked paragraph text frames (Text > Paragraph Text Frame > Link / Unlink).
//!
//! A chain is a list of paragraph frames; the text of the whole chain lives
//! in the frames in order and overflow flows from one frame into the next.
//! Links are kept in the object data of each frame (`text.next`,
//! `text.prev`, shape ids), so they survive save and load without a model
//! change. Re-flow runs after typing, linking and resizing a frame.

use crate::app::App;
use crate::i18n::tr;
use std::collections::HashMap;
use tracedraw_core::{
    document::text_outline::TextRequest,
    document::{Shape, ShapeKind, TextSpan},
    geometry::Point,
    Command, ShapeId,
};

/// The most pages Flow Into New Pages adds at once.
pub const MAX_AUTOFLOW_PAGES: usize = 500;

const NEXT: &str = "text.next";
const PREV: &str = "text.prev";

fn data_id(data: &[(String, String)], key: &str) -> Option<ShapeId> {
    data.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .map(ShapeId)
}

fn with_key(
    mut data: Vec<(String, String)>,
    key: &str,
    value: Option<ShapeId>,
) -> Vec<(String, String)> {
    data.retain(|(k, _)| k != key);
    if let Some(v) = value {
        data.push((key.to_string(), v.raw().to_string()));
    }
    data
}

/// Take the chain links out of a copied object's data: a copy is not in
/// its original's chain (see [`relink_copies`] for chains copied whole).
pub fn without_links(data: &mut Vec<(String, String)>) {
    data.retain(|(k, _)| k != NEXT && k != PREV);
}

/// Copies whose links were taken out, `copies[i]` made of `originals[i]`
/// with the same structure (groups and clip frames copied member by
/// member, in order): a link to a frame copied along points to its copy,
/// so a chain copied whole stays a chain of the copies; a link to any
/// other frame stays out.
pub fn relink_copies(originals: &[Shape], copies: &mut [Shape]) {
    fn map_ids(o: &Shape, c: &Shape, map: &mut HashMap<ShapeId, ShapeId>) {
        map.insert(o.id, c.id);
        match (&o.kind, &c.kind) {
            (ShapeKind::Group { children: a }, ShapeKind::Group { children: b }) => {
                for (x, y) in a.iter().zip(b) {
                    map_ids(x, y, map);
                }
            }
            (
                ShapeKind::ClipFrame {
                    frame: fa,
                    contents: a,
                },
                ShapeKind::ClipFrame {
                    frame: fb,
                    contents: b,
                },
            ) => {
                map_ids(fa, fb, map);
                for (x, y) in a.iter().zip(b) {
                    map_ids(x, y, map);
                }
            }
            _ => {}
        }
    }
    fn relink(o: &Shape, c: &mut Shape, map: &HashMap<ShapeId, ShapeId>) {
        for key in [NEXT, PREV] {
            if let Some(to) = data_id(&o.data, key).and_then(|id| map.get(&id)) {
                c.data = with_key(std::mem::take(&mut c.data), key, Some(*to));
            }
        }
        match (&o.kind, &mut c.kind) {
            (ShapeKind::Group { children: a }, ShapeKind::Group { children: b }) => {
                for (x, y) in a.iter().zip(b) {
                    relink(x, y, map);
                }
            }
            (
                ShapeKind::ClipFrame {
                    frame: fa,
                    contents: a,
                },
                ShapeKind::ClipFrame {
                    frame: fb,
                    contents: b,
                },
            ) => {
                relink(fa, fb, map);
                for (x, y) in a.iter().zip(b) {
                    relink(x, y, map);
                }
            }
            _ => {}
        }
    }
    let mut map = HashMap::new();
    for (o, c) in originals.iter().zip(copies.iter()) {
        map_ids(o, c, &mut map);
    }
    for (o, c) in originals.iter().zip(copies.iter_mut()) {
        relink(o, c, &map);
    }
}

impl App {
    fn frame_data(&self, id: ShapeId) -> Option<Vec<(String, String)>> {
        let s = self.doc().find_shape(id)?;
        matches!(s.kind, ShapeKind::Text { frame: Some(_), .. }).then(|| s.data.clone())
    }

    /// Next frame in the chain, if any and still present.
    pub fn next_frame(&self, id: ShapeId) -> Option<ShapeId> {
        let next = data_id(&self.frame_data(id)?, NEXT)?;
        self.frame_data(next).map(|_| next)
    }

    /// Previous frame in the chain, if any and still present.
    pub fn prev_frame(&self, id: ShapeId) -> Option<ShapeId> {
        let prev = data_id(&self.frame_data(id)?, PREV)?;
        self.frame_data(prev).map(|_| prev)
    }

    /// First frame of the chain containing `id`.
    pub fn chain_head(&self, id: ShapeId) -> ShapeId {
        let mut cur = id;
        let mut guard = 0;
        while let Some(p) = self.prev_frame(cur) {
            cur = p;
            guard += 1;
            if guard > 1000 {
                break;
            }
        }
        cur
    }

    /// All frames of the chain starting at its head, in flow order.
    pub fn chain_of(&self, id: ShapeId) -> Vec<ShapeId> {
        let mut out = vec![self.chain_head(id)];
        let mut guard = 0;
        while let Some(n) = self.next_frame(*out.last().unwrap_or(&id)) {
            if out.contains(&n) {
                break;
            }
            out.push(n);
            guard += 1;
            if guard > 1000 {
                break;
            }
        }
        out
    }

    pub fn is_linked_frame(&self, id: ShapeId) -> bool {
        self.next_frame(id).is_some() || self.prev_frame(id).is_some()
    }

    /// Link the two selected paragraph frames: the first selected flows into
    /// the second. The second frame's own text is appended to the chain.
    pub fn link_text_frames(&mut self) {
        let [a, b] = self.selection.as_slice() else {
            self.status = tr("status.link_needs_two_frames");
            return;
        };
        let (a, b) = (*a, *b);
        self.link_frames(a, b);
    }

    /// Link frame `a` to flow into `b` (on any page): false, with the
    /// reason on the status bar, when they cannot be linked.
    pub fn link_frames(&mut self, a: ShapeId, b: ShapeId) -> bool {
        let (Some(da), Some(db)) = (self.frame_data(a), self.frame_data(b)) else {
            self.status = tr("status.link_needs_two_frames");
            return false;
        };
        if a == b
            || self.chain_of(a).contains(&b)
            || self.next_frame(a).is_some()
            || self.prev_frame(b).is_some()
        {
            self.status = tr("status.link_already");
            return false;
        }
        let cmds = vec![
            Command::SetObjectData {
                shape: a,
                data: with_key(da, NEXT, Some(b)),
            },
            Command::SetObjectData {
                shape: b,
                data: with_key(db, PREV, Some(a)),
            },
        ];
        let _ = self.engine.run_batch("Link Frames", &cmds);
        self.reflow_chain(a);
        self.status = tr("status.frames_linked");
        true
    }

    /// The frame whose flow tab is under the screen point `at` (Pick
    /// tool): a frame shown with a tab that flows into no other frame.
    pub fn flow_tab_at(&self, at: egui::Pos2) -> Option<ShapeId> {
        self.shown_text_frames(None)
            .into_iter()
            .filter(|f| (f.overflow || f.chosen) && !f.linked)
            .find(|f| {
                let a = self.view.to_screen(f.corners[0]);
                let b = self.view.to_screen(f.corners[1]);
                crate::canvas::flow_tab_rect(a, b, self.handle_size())
                    .expand(2.0)
                    .contains(at)
            })
            .map(|f| f.id)
    }

    /// The frame whose top tab is under the screen point `at` (Pick
    /// tool): a picked frame that no other frame flows into yet.
    pub fn flow_in_tab_at(&self, at: egui::Pos2) -> Option<ShapeId> {
        self.shown_text_frames(None)
            .into_iter()
            .filter(|f| f.chosen && !f.receives)
            .find(|f| {
                let a = self.view.to_screen(f.corners[3]);
                let b = self.view.to_screen(f.corners[2]);
                crate::canvas::flow_in_tab_rect(a, b, self.handle_size())
                    .expand(2.0)
                    .contains(at)
            })
            .map(|f| f.id)
    }

    /// A click on a frame's flow tab: the next click says where its text
    /// flows on (#371).
    pub fn start_text_link(&mut self, from: ShapeId) {
        self.pending_text_link = Some(from);
        self.text_link_incoming = false;
        self.status = tr("status.flow_text_where");
    }

    /// A click on a frame's top tab: the next click says which frame's
    /// text flows into it (a new frame on empty space).
    pub fn start_text_link_in(&mut self, to: ShapeId) {
        self.pending_text_link = Some(to);
        self.text_link_incoming = true;
        self.status = tr("status.flow_text_from");
    }

    /// Let go of a pending link (Esc, a right click).
    pub fn cancel_text_link(&mut self) {
        self.pending_text_link = None;
        self.text_link_drag = None;
        self.text_link_incoming = false;
        self.text_link_from_tab = false;
    }

    /// The screen point the pending link's line starts from: the middle of
    /// the tab it came from.
    pub fn text_link_origin(&self) -> Option<egui::Pos2> {
        let id = self.pending_text_link?;
        let s = self.doc().find_shape(id)?;
        let c = s.text_frame_corners()?;
        let (a, b) = if self.text_link_incoming {
            (c[3], c[2])
        } else {
            (c[0], c[1])
        };
        let (a, b) = (self.view.to_screen(a), self.view.to_screen(b));
        Some(if self.text_link_incoming {
            crate::canvas::flow_in_tab_rect(a, b, self.handle_size()).center()
        } else {
            crate::canvas::flow_tab_rect(a, b, self.handle_size()).center()
        })
    }

    /// The click after a flow tab, at page point `p` (a drag: the frame
    /// `rect`): on another paragraph frame (on any page) the text flows
    /// into it; elsewhere a new frame takes it, the size of the first one
    /// with its top left corner at `p`, or the dragged rectangle.
    pub fn finish_text_link(&mut self, p: Point, rect: Option<tracedraw_core::geometry::Rect>) {
        let incoming = std::mem::take(&mut self.text_link_incoming);
        self.text_link_from_tab = false;
        let Some(from) = self.pending_text_link.take() else {
            return;
        };
        if rect.is_none() {
            if let Some(other) = self.hit_test(p).filter(|id| self.frame_data(*id).is_some()) {
                if other != from {
                    // From a top tab the frame clicked flows into the
                    // frame of the tab.
                    let (a, b) = if incoming {
                        (other, from)
                    } else {
                        (from, other)
                    };
                    self.link_frames(a, b);
                    self.select(vec![other]);
                    return;
                }
            }
        }
        let Some(src) = self.doc().find_shape(from).cloned() else {
            return;
        };
        let ShapeKind::Text {
            spans,
            frame: Some(size),
            align,
            para,
            ..
        } = &src.kind
        else {
            return;
        };
        let (origin, size) = match rect {
            Some(r) if r.width() > 1.0 && r.height() > 1.0 => (
                Point::new(r.x0, r.y0),
                tracedraw_core::geometry::Size::new(r.width(), r.height()),
            ),
            _ => (Point::new(p.x, p.y - size.height), *size),
        };
        let style = spans.last().cloned().map(|mut s| {
            s.text.clear();
            s
        });
        let Some(span) = style else {
            return;
        };
        let kind = ShapeKind::Text {
            spans: vec![span],
            origin,
            frame: Some(size),
            align: *align,
            para: para.clone(),
            on_path: None,
        };
        let Some(to) = self.new_shape(kind) else {
            return;
        };
        if incoming {
            // The new frame comes first: the text starts in it.
            self.link_frames(to, from);
        } else {
            self.link_frames(from, to);
        }
        self.select(vec![to]);
    }

    /// Arcano edition, Page > Threaded Text Frames: on the pages from
    /// index `from` to `to` (in range), a paragraph text frame inside the
    /// margins `[top, bottom, inner, outer]` (inner and outer swapped on
    /// left-hand pages when the pages face each other, the first page a
    /// right-hand one), with `columns` columns `gutter` mm apart, each
    /// linked to the one before: text put in the first runs on through
    /// the pages (and Flow Into New Pages carries it on). One undo step;
    /// the first frame, which is selected.
    pub fn thread_page_frames(
        &mut self,
        from: usize,
        to: usize,
        margins: [f64; 4],
        columns: u32,
        gutter: f64,
    ) -> Option<ShapeId> {
        let n = self.doc().pages.len();
        if n == 0 || from > to || from >= n {
            return None;
        }
        let to = to.min(n - 1);
        let [top, bottom, inner, outer] = margins.map(|m| if m.is_finite() { m.max(0.0) } else { 0.0 });
        let facing = self.doc().metadata.facing_pages;
        let span = TextSpan {
            bold: self.text_bold,
            italic: self.text_italic,
            ..TextSpan::new("", self.text_font.clone(), self.text_size_pt)
        };
        let mut para = tracedraw_core::ParagraphStyle::default();
        para.hyphenate = self.text_hyphenation;
        para.columns = columns.clamp(1, 20);
        para.gutter = if gutter.is_finite() { gutter.max(0.0) } else { 0.0 };
        let keep_page = self.page;
        let keep_layer = self.picked_layer;
        let mark = self.engine.begin_group();
        let mut made: Vec<ShapeId> = Vec::new();
        for i in from..=to {
            let Some(page) = self.doc().pages.get(i).map(|p| (p.id, p.size)) else {
                break;
            };
            let (id, size) = page;
            let right_hand = !facing || i % 2 == 0;
            let (left, right) = if right_hand { (inner, outer) } else { (outer, inner) };
            let w = (size.width - left - right).max(1.0);
            let h = (size.height - top - bottom).max(1.0);
            self.page = id;
            self.picked_layer = None;
            let kind = ShapeKind::Text {
                spans: vec![span.clone()],
                origin: Point::new(left, bottom),
                frame: Some(tracedraw_core::geometry::Size::new(w, h)),
                align: self.text_align,
                para: para.clone(),
                on_path: None,
            };
            if let Some(f) = self.new_shape(kind) {
                if let Some(&prev) = made.last() {
                    self.link_frames(prev, f);
                }
                made.push(f);
            }
        }
        self.engine.end_group(mark, "Threaded Text Frames");
        self.page = keep_page;
        self.picked_layer = keep_layer;
        let first = made.first().copied();
        if let Some(f) = first {
            if let Some(i) = self.page_index_of(f) {
                self.goto_page(i);
            }
            self.select(vec![f]);
        }
        first
    }

    /// Arcano edition, Text > Flow Into New Pages: while the last frame of
    /// the chain of `any` (a paragraph frame) overflows, a page is added
    /// after the last frame's page, a frame put where the last one is
    /// on its page and linked to it, so the text runs on until it all
    /// fits (at most [`MAX_AUTOFLOW_PAGES`] pages). One undo step. The
    /// number of pages added.
    pub fn autoflow(&mut self, any: ShapeId) -> usize {
        let mark = self.engine.begin_group();
        let mut added = 0;
        while added < MAX_AUTOFLOW_PAGES {
            let Some(&last) = self.chain_of(any).last() else {
                break;
            };
            let overflows = self
                .text_layout_of(last)
                .is_some_and(|(l, _)| l.overflow);
            if !overflows {
                break;
            }
            let Some(src) = self.doc().find_shape(last).cloned() else {
                break;
            };
            let ShapeKind::Text {
                spans,
                origin,
                frame: Some(size),
                align,
                para,
                ..
            } = &src.kind
            else {
                break;
            };
            let Some(page) = self.page_index_of(last) else {
                break;
            };
            if self.page_index() != page {
                self.goto_page(page);
            }
            self.insert_page(true);
            let Some(span) = spans.last().cloned().map(|mut s| {
                s.text.clear();
                s
            }) else {
                break;
            };
            // Facing pages: the new page faces the other way, so the frame
            // keeps its margins mirrored (inner stays inner).
            let mut origin = *origin;
            if self.doc().metadata.facing_pages && src.transform == tracedraw_core::Affine::IDENTITY {
                if let Some(w) = self.doc().pages.get(page + 1).map(|p| p.size.width) {
                    origin.x = w - origin.x - size.width;
                }
            }
            let kind = ShapeKind::Text {
                spans: vec![span],
                origin,
                frame: Some(*size),
                align: *align,
                para: para.clone(),
                on_path: None,
            };
            let Some(to) = self.new_shape(kind) else {
                break;
            };
            // A turned or scaled frame: the new one the same way.
            if self.doc().find_shape(to).is_some_and(|s| s.transform != src.transform) {
                self.run(Command::TransformShapes {
                    shapes: vec![to],
                    transform: src.transform,
                });
            }
            if !self.link_frames(last, to) {
                break;
            }
            added += 1;
        }
        self.engine.end_group(mark, "Flow Into New Pages");
        self.status = tr(if added == 0 { "status.autoflow_none" } else { "status.autoflow_done" })
            .replace("{n}", &added.to_string());
        added
    }

    /// The index of the page a shape is on (master layers aside).
    pub fn page_index_of(&self, id: ShapeId) -> Option<usize> {
        fn holds(s: &tracedraw_core::Shape, id: ShapeId) -> bool {
            s.id == id
                || match &s.kind {
                    ShapeKind::Group { children } => children.iter().any(|c| holds(c, id)),
                    ShapeKind::ClipFrame { frame, contents } => {
                        holds(frame, id) || contents.iter().any(|c| holds(c, id))
                    }
                    _ => false,
                }
        }
        self.doc()
            .pages
            .iter()
            .position(|p| p.layers.iter().any(|l| l.shapes.iter().any(|s| holds(s, id))))
    }

    /// Unlink the selected frames from their neighbours; each keeps the text
    /// it currently shows.
    pub fn unlink_text_frames(&mut self) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            let Some(d) = self.frame_data(id) else {
                continue;
            };
            let next = data_id(&d, NEXT);
            let prev = data_id(&d, PREV);
            if next.is_none() && prev.is_none() {
                continue;
            }
            cmds.push(Command::SetObjectData {
                shape: id,
                data: with_key(with_key(d, NEXT, None), PREV, None),
            });
            if let Some(n) = next {
                if let Some(dn) = self.frame_data(n) {
                    cmds.push(Command::SetObjectData {
                        shape: n,
                        data: with_key(dn, PREV, None),
                    });
                }
            }
            if let Some(p) = prev {
                if let Some(dp) = self.frame_data(p) {
                    cmds.push(Command::SetObjectData {
                        shape: p,
                        data: with_key(dp, NEXT, None),
                    });
                }
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Unlink Frames", &cmds);
        }
    }

    /// Distribute the chain's text over its frames: each frame keeps what
    /// fits, the rest flows on; the last frame takes whatever is left.
    pub fn reflow_chain(&mut self, any: ShapeId) {
        let chain = self.chain_of(any);
        if chain.len() < 2 {
            return;
        }
        // Gather the whole text in chain order.
        let mut all: Vec<TextSpan> = Vec::new();
        let mut kinds = Vec::new();
        for id in &chain {
            let Some(s) = self.doc().find_shape(*id) else {
                return;
            };
            if let ShapeKind::Text { spans, .. } = &s.kind {
                all.extend(spans.iter().cloned());
            }
            kinds.push(s.kind.clone());
        }
        let fonts = tracedraw_text::fonts();
        let mut cmds = Vec::new();
        let mut rest = all;
        let n = chain.len();
        for (i, (id, kind)) in chain.iter().zip(kinds).enumerate() {
            let ShapeKind::Text {
                origin,
                frame,
                align,
                para,
                on_path,
                spans: old,
            } = kind
            else {
                continue;
            };
            let mine = if i + 1 == n {
                std::mem::take(&mut rest)
            } else {
                let layout = fonts.layout(&TextRequest {
                    spans: &rest,
                    frame,
                    align,
                    para: &para,
                    on_path: on_path.as_ref(),
                });
                // Widows, orphans and titles kept with what follows.
                let lines: Vec<(usize, usize)> = layout
                    .lines
                    .iter()
                    .map(|l| (l.start_char, l.end_char))
                    .collect();
                let at = tracedraw_core::paragraphs::keep_split(
                    &rest,
                    align,
                    &para,
                    &lines,
                    layout.fitted_chars,
                );
                let (head, tail) = tracedraw_core::split_spans_at(&rest, at);
                rest = tail;
                head
            };
            if mine == old {
                continue;
            }
            cmds.push(Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::Text {
                    spans: mine,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Text Flow", &cmds);
        }
    }

    /// The paragraph format at the caret (else the first paragraph) of
    /// text `id`, for its rules between frames.
    pub fn keep_rules_at(
        &self,
        id: ShapeId,
    ) -> Option<tracedraw_core::paragraphs::ParagraphFormat> {
        let s = self.doc().find_shape(id)?;
        let ShapeKind::Text {
            spans, align, para, ..
        } = &s.kind
        else {
            return None;
        };
        let at = self.paragraph_range(id).map_or(0, |r| r.0);
        let i = tracedraw_core::paragraphs::paragraph_of(spans, at);
        Some(tracedraw_core::paragraphs::paragraph_format(
            spans, *align, para, i,
        ))
    }

    /// Give the paragraphs at the caret (else all of text `id`) these
    /// rules, then flow its chain again.
    pub fn set_keep_rules(&mut self, id: ShapeId, orphans: u8, widows: u8, keep: bool) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        let mut kind = s.kind.clone();
        tracedraw_core::paragraphs::edit_paragraph_formats(
            &mut kind,
            self.paragraph_range(id),
            |f| {
                f.orphans = orphans;
                f.widows = widows;
                f.keep_with_next = keep;
            },
        );
        if kind != s.kind {
            self.run(Command::SetShapeKind { shape: id, kind });
            self.reflow_chain(id);
        }
    }

    /// Re-flow every chain that touches the given shapes (after a resize).
    pub fn reflow_chains_of(&mut self, ids: &[ShapeId]) {
        let mut done: Vec<ShapeId> = Vec::new();
        for id in ids {
            if !self.is_linked_frame(*id) {
                continue;
            }
            let head = self.chain_head(*id);
            if done.contains(&head) {
                continue;
            }
            done.push(head);
            self.reflow_chain(head);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flow Into New Pages adds pages until the text fits, each with a
    /// frame where the last one was, linked; one undo step takes it back.
    #[test]
    fn autoflow_adds_pages_until_the_text_fits() {
        let mut app = App::headless();
        let text = "Palavras que enchem a moldura e seguem adiante. ".repeat(45);
        let id = frame(&mut app, &text, 60.0, 40.0);
        let pages = app.doc().pages.len();
        let steps = app.engine.history_labels().0.len();
        let added = app.autoflow(id);
        assert!(added >= 2, "{added}");
        assert_eq!(app.doc().pages.len(), pages + added);
        let chain = app.chain_of(id);
        assert_eq!(chain.len(), added + 1);
        let last = *chain.last().unwrap();
        assert!(!app.text_layout_of(last).unwrap().0.overflow);
        assert_eq!(app.page_index_of(last), Some(added));
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        // Nothing overflows: nothing added.
        assert_eq!(app.autoflow(id), 0);
        app.undo();
        assert_eq!(app.doc().pages.len(), pages);
    }

    #[test]
    fn threaded_frames_sit_in_the_margins_and_carry_the_text_on() {
        let mut app = App::headless();
        for _ in 0..3 {
            app.add_page();
        }
        app.set_facing_pages(true);
        let w = app.doc().pages[0].size.width;
        let steps = app.engine.history_labels().0.len();
        let first = app
            .thread_page_frames(0, 3, [20.0, 25.0, 22.0, 15.0], 2, 5.0)
            .expect("frames");
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let chain = app.chain_of(first);
        assert_eq!(chain.len(), 4);
        let frame = |app: &App, id| match &app.doc().find_shape(id).unwrap().kind {
            ShapeKind::Text { origin, frame: Some(f), para, .. } => (origin.x, f.width, para.columns),
            _ => panic!("a frame"),
        };
        // Page 1 (right-hand): the inner margin on the left; page 2
        // (left-hand): the outer one.
        assert_eq!(frame(&app, chain[0]), (22.0, w - 37.0, 2));
        assert_eq!(frame(&app, chain[1]).0, 15.0);
        // Each on its own page, in order.
        let pages: Vec<usize> = chain.iter().map(|id| app.page_index_of(*id).unwrap()).collect();
        assert_eq!(pages, vec![0, 1, 2, 3]);
        app.undo();
        assert!(app.doc().find_shape(first).is_none());
    }

    #[test]
    fn autoflow_mirrors_the_frames_on_facing_pages() {
        let mut app = App::headless();
        app.set_facing_pages(true);
        let width = app.doc().pages[0].size.width;
        let text = "Palavras que enchem a moldura e seguem adiante. ".repeat(45);
        // 10 mm from the left (the spine of a right-hand page), 60 wide.
        let id = frame(&mut app, &text, 60.0, 40.0);
        assert!(app.autoflow(id) >= 2);
        let x = |app: &App, id: ShapeId| match &app.doc().find_shape(id).unwrap().kind {
            ShapeKind::Text { origin, .. } => origin.x,
            _ => f64::NAN,
        };
        let chain = app.chain_of(id);
        // Left-hand pages: 10 mm from the right; right-hand ones as the first.
        assert!((x(&app, chain[1]) - (width - 10.0 - 60.0)).abs() < 1e-9);
        assert!((x(&app, chain[2]) - 10.0).abs() < 1e-9);
    }
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::TextAlign;

    fn frame(app: &mut App, text: &str, w: f64, h: f64) -> ShapeId {
        let kind = ShapeKind::Text {
            spans: vec![TextSpan::new(text, "DejaVu Sans", 12.0)],
            origin: Point::new(10.0, 10.0),
            frame: Some(Size::new(w, h)),
            align: TextAlign::Left,
            para: Default::default(),
            on_path: None,
        };
        app.new_shape(kind).unwrap()
    }

    #[test]
    fn linking_flows_overflow_into_the_second_frame() {
        tracedraw_text::install();
        let mut app = App::headless();
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
        let a = frame(&mut app, long, 30.0, 8.0);
        let b = frame(&mut app, "", 60.0, 60.0);
        app.select(vec![a, b]);
        app.link_text_frames();
        assert_eq!(app.next_frame(a), Some(b));
        let text_of = |app: &App, id: ShapeId| -> String {
            match &app.doc().find_shape(id).unwrap().kind {
                ShapeKind::Text { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
                _ => String::new(),
            }
        };
        let ta = text_of(&app, a);
        let tb = text_of(&app, b);
        assert!(!ta.is_empty() && !tb.is_empty(), "a={ta:?} b={tb:?}");
        assert_eq!(format!("{ta}{tb}"), long);
        // Unlinking keeps the split text in place.
        app.select(vec![a]);
        app.unlink_text_frames();
        assert_eq!(app.next_frame(a), None);
        assert_eq!(text_of(&app, a), ta);
    }

    /// The flow tab (#371): a click on it, then a click on another page's
    /// empty space makes a frame there that the text flows into; a click
    /// on a frame links to that frame.
    #[test]
    fn the_flow_tab_links_to_a_new_frame_on_another_page() {
        tracedraw_text::install();
        let mut app = App::headless();
        app.canvas_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let page = app.page_rect();
        app.view.fit(page, app.canvas_rect);
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
        let a = frame(&mut app, long, 30.0, 8.0);
        app.select(vec![a]);
        // The tab hangs under the bottom edge's middle (10 to 40 at y 10).
        let edge = app.view.to_screen(Point::new(25.0, 10.0));
        let tab = crate::canvas::flow_tab_rect(
            app.view.to_screen(Point::new(10.0, 10.0)),
            app.view.to_screen(Point::new(40.0, 10.0)),
            app.handle_size(),
        );
        assert!(tab.top() > edge.y);
        assert_eq!(app.flow_tab_at(tab.center()), Some(a));
        app.start_text_link(a);
        assert_eq!(app.pending_text_link, Some(a));
        // On to page 2, a click on its empty space.
        let first_page = app.page;
        app.add_page();
        assert_ne!(app.page, first_page);
        app.finish_text_link(Point::new(50.0, 150.0), None);
        assert!(app.pending_text_link.is_none());
        let b = app.next_frame(a).expect("a new frame");
        let s = app.doc().find_shape(b).cloned().expect("frame");
        let corners = s.text_frame_corners().expect("a frame");
        // Its top left at the click, the first frame's size.
        assert!((corners[3].x - 50.0).abs() < 1e-9 && (corners[3].y - 150.0).abs() < 1e-9);
        let text = |id: ShapeId| match &app.doc().find_shape(id).unwrap().kind {
            ShapeKind::Text { spans, .. } => {
                spans.iter().map(|s| s.text.as_str()).collect::<String>()
            }
            _ => String::new(),
        };
        assert!(!text(b).is_empty());
        assert_eq!(format!("{}{}", text(a), text(b)), long);
        // A frame that flows on has no tab to click.
        app.page = first_page;
        app.select(vec![a]);
        assert_eq!(app.flow_tab_at(tab.center()), None);
    }

    /// A copy of one frame of a chain is a frame of its own: editing it
    /// does not flow into the original's next frame. A chain copied whole
    /// is a chain of the copies.
    #[test]
    fn copies_leave_the_chain_unless_copied_whole() {
        tracedraw_text::install();
        let mut app = App::headless();
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
        let a = frame(&mut app, long, 30.0, 8.0);
        let b = frame(&mut app, "", 60.0, 60.0);
        app.select(vec![a, b]);
        app.link_text_frames();
        let text_of = |app: &App, id: ShapeId| -> String {
            match &app.doc().find_shape(id).unwrap().kind {
                ShapeKind::Text { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
                _ => String::new(),
            }
        };
        let tb = text_of(&app, b);
        // One frame copied: the copy is linked to nothing, and typing in
        // it leaves the original's chain alone.
        app.select(vec![a]);
        app.duplicate();
        let a2 = app.selection[0];
        assert_ne!(a2, a);
        assert!(!app.is_linked_frame(a2));
        app.begin_text_edit(a2);
        app.text_insert("more words here and there ");
        assert_eq!(
            text_of(&app, b),
            tb,
            "the original's next frame is untouched"
        );
        assert_eq!(app.next_frame(a), Some(b));
        // The copy pasted: the same.
        app.select(vec![b]);
        app.copy();
        app.paste();
        let b2 = app.selection[0];
        assert!(!app.is_linked_frame(b2));
        // The whole chain copied: the copies are a chain of their own.
        app.select(vec![a, b]);
        app.duplicate();
        let (a3, b3) = (app.selection[0], app.selection[1]);
        assert_eq!(app.next_frame(a3), Some(b3));
        assert_eq!(app.chain_of(b3), vec![a3, b3]);
        assert_eq!(app.next_frame(a), Some(b), "the original chain stays");
        // Copies of a copied chain are made with every other copy (reid):
        // nothing links back to the originals.
        app.select(vec![a3]);
        app.duplicate();
        assert!(!app.is_linked_frame(app.selection[0]));
    }

    /// Orphans and widows: a paragraph keeps two lines at the foot of the
    /// first frame and carries two to the next, and a title stays with
    /// the paragraph after it.
    #[test]
    fn widows_orphans_and_titles_kept_with_the_next_paragraph() {
        tracedraw_text::install();
        let text_of = |app: &App, id: ShapeId| -> String {
            match &app.doc().find_shape(id).unwrap().kind {
                ShapeKind::Text { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
                _ => String::new(),
            }
        };
        let para2 = "Era uma vez um livro com muitas linhas que precisavam ser divididas entre duas caixas de texto ligadas, sem deixar linha sozinha no pé nem no topo da página seguinte.";
        let text = format!("Um parágrafo curto.\nTítulo\n{para2}");
        let run = |height: f64, rules: bool| -> (String, String) {
            let mut app = App::headless();
            let a = frame(&mut app, "", 40.0, height);
            let b = frame(&mut app, "", 40.0, 200.0);
            let mut k = app.doc().find_shape(a).unwrap().kind.clone();
            if let ShapeKind::Text { spans, .. } = &mut k {
                *spans = vec![TextSpan::new(&text, "DejaVu Sans", 12.0)];
            }
            if rules {
                tracedraw_core::paragraphs::edit_paragraph_formats(&mut k, None, |f| {
                    f.widows = 2;
                    f.orphans = 2;
                });
                let t0 = "Um parágrafo curto.\n".chars().count();
                tracedraw_core::paragraphs::edit_paragraph_formats(&mut k, Some((t0, t0)), |f| {
                    f.keep_with_next = true
                });
            }
            app.run(Command::SetShapeKind { shape: a, kind: k });
            app.select(vec![a, b]);
            app.link_text_frames();
            (text_of(&app, a), text_of(&app, b))
        };
        let lines_in = |s: &str| -> usize {
            let spans = vec![TextSpan::new(s, "DejaVu Sans", 12.0)];
            tracedraw_text::fonts()
                .layout(&TextRequest {
                    spans: &spans,
                    frame: Some(tracedraw_core::geometry::Size::new(40.0, 500.0)),
                    align: TextAlign::Left,
                    para: &Default::default(),
                    on_path: None,
                })
                .lines
                .len()
        };
        let mut title_alone_without_rules = false;
        for h in (8..60).map(|h| h as f64) {
            let (pa, _) = run(h, false);
            title_alone_without_rules |= pa.ends_with("Título\n");
            let (ra, rb) = run(h, true);
            assert_eq!(format!("{ra}{rb}"), text);
            // The title never ends the frame alone.
            assert!(!ra.ends_with("Título\n"), "h={h}: {ra:?}");
            // A split paragraph leaves two lines on each side.
            if let Some(cut) = ra.find("Era") {
                let here = &ra[cut..];
                if !here.is_empty() && !rb.is_empty() {
                    assert!(lines_in(here) >= 2, "h={h}: orphan {here:?}");
                    assert!(
                        lines_in(rb.trim_start_matches("Título\n")) >= 2 || rb.is_empty(),
                        "h={h}: widow {rb:?}"
                    );
                }
            }
        }
        assert!(
            title_alone_without_rules,
            "some height leaves the title alone without the rules"
        );
    }
}
