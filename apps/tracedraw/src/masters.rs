//! Arcano edition: named master pages (A-Master, B-Chapter opening) and
//! the page operations the Pages panel offers on them: new masters,
//! their options, applying a master to pages, pages that keep the master
//! of the page they come after. The masters themselves are
//! `tracedraw_core::document::MasterPage`; see docs/behavior/pages-panel.md.

use crate::app::App;
use crate::i18n::tr;
use tracedraw_core::document::{MasterPage, MasterScope, MasterView, NO_MASTER};
use tracedraw_core::{Command, PageId};

/// The label of a master page as the panel shows it ("A-Master").
pub fn label(m: &MasterPage) -> String {
    let name = if m.name.trim().is_empty() {
        tr("master.default_name")
    } else {
        m.name.clone()
    };
    format!("{}-{}", m.prefix, name)
}

/// The prefix a new master gets: the first letter from B on that no
/// master uses, then two letters.
pub fn free_prefix(masters: &[MasterPage]) -> String {
    let taken = |p: &str| masters.iter().any(|m| m.prefix.eq_ignore_ascii_case(p));
    for c in 'A'..='Z' {
        let p = c.to_string();
        if !taken(&p) {
            return p;
        }
    }
    for a in 'A'..='Z' {
        for b in 'A'..='Z' {
            let p = format!("{a}{b}");
            if !taken(&p) {
                return p;
            }
        }
    }
    String::from("M")
}

impl App {
    /// The master pages, A first.
    pub fn masters(&self) -> Vec<MasterPage> {
        self.doc().masters()
    }

    /// Make a master page (one undo step) and return its id.
    pub fn new_master(&mut self, prefix: &str, name: &str, based_on: Option<u32>) -> Option<u32> {
        let prefix = if prefix.trim().is_empty() {
            free_prefix(&self.masters())
        } else {
            prefix.trim().to_string()
        };
        let layer_name = format!("{prefix}-{}", if name.trim().is_empty() {
            tr("master.default_name")
        } else {
            name.trim().to_string()
        });
        let before: Vec<u32> = self.masters().iter().map(|m| m.id).collect();
        self.run(Command::NewMasterPage {
            prefix,
            name: name.trim().to_string(),
            based_on,
            layer_name,
        });
        self.masters()
            .into_iter()
            .map(|m| m.id)
            .find(|id| !before.contains(id))
    }

    /// Change a master page's prefix, name or parent; false when refused
    /// (a parent that makes a loop).
    pub fn set_master(&mut self, master: MasterPage) -> bool {
        if self.doc().master_page(master.id).as_ref() == Some(&master) {
            return true;
        }
        if self.doc().master_would_loop(master.id, master.based_on) {
            self.status = tr("status.master_loop");
            return false;
        }
        self.run(Command::SetMasterPage { master });
        true
    }

    /// Delete master page `id` (never A) with its objects.
    pub fn delete_master(&mut self, id: u32) {
        if id != 0 && id != NO_MASTER {
            self.run(Command::DeleteMasterPage { id });
        }
    }

    /// Give `pages` the master `master` (one undo step).
    pub fn apply_master(&mut self, pages: &[PageId], master: u32) {
        let pages: Vec<PageId> = pages
            .iter()
            .copied()
            .filter(|p| self.doc().master_of(*p) != master)
            .collect();
        if !pages.is_empty() {
            self.run(Command::SetPageMaster { pages, master });
        }
    }

    /// Master `id` for the pages from index `a` to `b` (both 0-based, in
    /// range).
    #[cfg(test)]
    pub fn apply_master_to_range(&mut self, a: usize, b: usize, master: u32) {
        let ids: Vec<PageId> = self.doc().pages.iter().map(|p| p.id).collect();
        if ids.is_empty() {
            return;
        }
        let (a, b) = (a.min(b), a.max(b).min(ids.len() - 1));
        self.apply_master(&ids[a..=b], master);
    }

    /// A page just made after (or for) page `from` takes its master, as
    /// part of the step that made it.
    pub(crate) fn inherit_master(&mut self, new: PageId, from: PageId) {
        let m = self.doc().master_of(from);
        if m != 0 {
            self.run(Command::SetPageMaster {
                pages: vec![new],
                master: m,
            });
        }
    }

    /// Pages > Insert Pages: `count` pages with master `master`, at index
    /// `at` (0 = before the first page, the number of pages = at the end),
    /// in one undo step; the first new page becomes the current one.
    pub fn insert_pages(&mut self, count: usize, at: usize, master: u32) {
        let count = count.clamp(1, 9999);
        let mark = self.engine.begin_group();
        let size = self.page_size();
        let mut made = Vec::with_capacity(count);
        for k in 0..count {
            self.run(self.localized_page(size));
            let Some(new) = self.doc().pages.last().map(|p| p.id) else {
                break;
            };
            let to = (at + k).min(self.doc().pages.len() - 1);
            if to + 1 < self.doc().pages.len() {
                self.run(Command::MovePage { page: new, to });
            }
            made.push(new);
        }
        if master != 0 && !made.is_empty() {
            self.run(Command::SetPageMaster {
                pages: made.clone(),
                master,
            });
        }
        self.engine.end_group(mark, "Insert Pages");
        if let Some(i) = made
            .first()
            .and_then(|p| self.doc().pages.iter().position(|q| q.id == *p))
        {
            self.goto_page(i);
        }
    }

    /// Move the pages `pages` (in their order in the drawing) so the first
    /// of them is at index `to` among the pages that are not moved (0 =
    /// first); one undo step.
    pub fn move_pages(&mut self, pages: &[PageId], to: usize) {
        let order: Vec<PageId> = self.doc().pages.iter().map(|p| p.id).collect();
        let moving: Vec<PageId> = order.iter().copied().filter(|p| pages.contains(p)).collect();
        if moving.is_empty() {
            return;
        }
        let mut rest: Vec<PageId> = order.iter().copied().filter(|p| !pages.contains(p)).collect();
        let to = to.min(rest.len());
        let tail = rest.split_off(to);
        rest.extend(moving.iter().copied());
        rest.extend(tail);
        if rest == order {
            return;
        }
        let mark = self.engine.begin_group();
        for (i, p) in rest.iter().enumerate() {
            if self.doc().pages.iter().position(|q| q.id == *p) != Some(i) {
                self.run(Command::MovePage { page: *p, to: i });
            }
        }
        self.engine.end_group(mark, "Move Pages");
    }

    /// Delete the pages `pages` (never all of them) in one undo step.
    pub fn delete_pages_by_id(&mut self, pages: &[PageId]) {
        let ids: Vec<PageId> = self.doc().pages.iter().map(|p| p.id).collect();
        let gone: Vec<PageId> = ids.iter().copied().filter(|p| pages.contains(p)).collect();
        if gone.is_empty() || gone.len() >= ids.len() {
            return;
        }
        let cmds: Vec<Command> = gone
            .iter()
            .map(|&page| Command::DeletePage { page })
            .collect();
        if let Err(e) = self.engine.run_batch("Delete Page", &cmds) {
            self.status = e.to_string();
            return;
        }
        if gone.contains(&self.page) {
            let first_left = ids
                .iter()
                .position(|p| gone.contains(p))
                .and_then(|i| ids[i..].iter().find(|p| !gone.contains(p)).or(ids[..i].last()))
                .copied();
            if let Some(p) = first_left {
                self.page = p;
            }
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    /// Edit master page `id` in place of the pages: the current page shows
    /// only that master (its right-hand page first) and new objects go on
    /// its layer for that side.
    pub fn edit_master(&mut self, id: u32) {
        if self.doc().master_page(id).is_none() || !self.has_document() {
            return;
        }
        if self.multipage {
            self.set_multipage(false);
        }
        self.selection.clear();
        self.master_view = Some(MasterView { master: id, left: false });
        self.pick_master_layer();
        self.objects_master_closed = false;
        self.fit_pending = true;
        let m = self.doc().master_page(id).map(|m| label(&m)).unwrap_or_default();
        self.status = crate::i18n::trf("status.editing_master", &[("m", &m)]);
    }

    /// Show the left-hand (true) or right-hand page of the master being
    /// edited.
    pub fn set_master_side(&mut self, left: bool) {
        if let Some(v) = self.master_view.as_mut() {
            if v.left != left {
                v.left = left;
                self.selection.clear();
                self.pick_master_layer();
            }
        }
    }

    /// A press at screen point `s` on the master's other page (the one
    /// shown beside the page edited): that page becomes the one edited,
    /// where it is on screen, and the press goes on there (drawing,
    /// picking). True when it was there.
    pub fn master_press(&mut self, s: egui::Pos2) -> bool {
        let Some(v) = self.master_view else { return false };
        let Some(r) = crate::canvas::master_companion_rect(self) else {
            return false;
        };
        if !r.contains(s) {
            return false;
        }
        let paper = crate::canvas::page_screen_rect(self);
        self.view.origin.x += r.left() - paper.left();
        self.set_master_side(!v.left);
        true
    }

    /// Back to the pages from a master being edited.
    pub fn leave_master(&mut self) {
        if self.master_view.take().is_some() {
            self.selection.clear();
            self.picked_layer = None;
            self.fit_pending = true;
            self.status.clear();
        }
    }

    /// The layer new objects go to while a master is edited: with facing
    /// pages, the master's layer for the side shown (made when it has
    /// none: objects drawn on a left-hand master page are on left-hand
    /// pages only); otherwise its layer for every page.
    fn pick_master_layer(&mut self) {
        let Some(v) = self.master_view else { return };
        let facing = self.doc().metadata.facing_pages;
        let want = if !facing {
            MasterScope::All
        } else if v.left {
            MasterScope::Even
        } else {
            MasterScope::Odd
        };
        let find = |app: &App| {
            app.doc()
                .master
                .iter()
                .find(|l| l.master_page == v.master && l.scope == want)
                .map(|l| l.id)
        };
        let mut layer = find(self);
        if layer.is_none() {
            let name = self.doc().master_page(v.master).map(|m| label(&m)).unwrap_or_default();
            let name = match want {
                MasterScope::Even => crate::i18n::trf("master.layer_left", &[("m", &name)]),
                MasterScope::Odd => crate::i18n::trf("master.layer_right", &[("m", &name)]),
                _ => name,
            };
            let before: Vec<tracedraw_core::LayerId> = self.doc().master.iter().map(|l| l.id).collect();
            let mark = self.engine.begin_group();
            self.run(Command::AddMasterLayer { name, scope: want });
            let new = self.doc().master.iter().map(|l| l.id).find(|l| !before.contains(l));
            if let Some(l) = new {
                self.run(Command::SetLayerMasterPage {
                    layer: l,
                    master: v.master,
                });
            }
            self.engine.end_group(mark, "Edit Master");
            layer = find(self);
        }
        self.picked_layer = layer;
    }

    /// Save page `page` as a new master: its objects copied onto the new
    /// master's layer; the page itself unchanged. Returns the master.
    pub fn page_as_master(&mut self, page: PageId) -> Option<u32> {
        let shapes: Vec<tracedraw_core::Shape> = self
            .doc()
            .page(page)
            .ok()?
            .layers
            .iter()
            .flat_map(|l| l.shapes.iter().cloned())
            .collect();
        let mark = self.engine.begin_group();
        let id = self.new_master("", "", None);
        if let Some(id) = id {
            let layer = self
                .doc()
                .master
                .iter()
                .find(|l| l.master_page == id)
                .map(|l| l.id);
            if let Some(layer) = layer {
                for s in shapes {
                    let fresh = self.engine.new_shape_id();
                    let s = crate::app::reid_pub(s, fresh, &mut self.engine);
                    self.run(Command::AddShape { layer, shape: s });
                }
            }
        }
        self.engine.end_group(mark, "Save as Master");
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_master_on_pages_and_new_pages_after_them_keep_it() {
        let mut app = App::headless();
        for _ in 0..3 {
            app.add_page();
        }
        let pages: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        let b = app.new_master("", "Abertura", Some(0)).expect("B");
        assert_eq!(app.masters()[1].prefix, "B");
        assert_eq!(label(&app.masters()[1]), "B-Abertura");
        let steps = app.engine.history_labels().0.len();
        app.apply_master_to_range(1, 2, b);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        assert_eq!(app.doc().master_of(pages[1]), b);
        assert_eq!(app.doc().master_of(pages[3]), 0);
        // A page inserted after a B page is a B page, in the same step.
        app.goto_page(2);
        let steps = app.engine.history_labels().0.len();
        app.insert_page(true);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        assert_eq!(app.doc().master_of(app.page), b);
        // Three pages with no master at the start.
        app.insert_pages(3, 0, NO_MASTER);
        let first: Vec<u32> = app.doc().pages[..3]
            .iter()
            .map(|p| app.doc().master_of(p.id))
            .collect();
        assert_eq!(first, vec![NO_MASTER; 3]);
        assert_eq!(app.page_index(), 0);
        // Moving two pages to the end keeps their order.
        let order: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        app.move_pages(&[order[0], order[1]], usize::MAX);
        let now: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        assert_eq!(&now[now.len() - 2..], &[order[0], order[1]]);
        app.undo();
        let back: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        assert_eq!(back, order);
        // Deleting B leaves its pages with no master.
        app.delete_master(b);
        assert_eq!(app.doc().master_of(pages[1]), NO_MASTER);
    }

    #[test]
    fn what_is_drawn_on_a_master_shows_on_its_pages_on_its_side() {
        let mut app = App::headless();
        for _ in 0..3 {
            app.add_page();
        }
        app.set_facing_pages(true);
        app.set_multipage(false);
        let pages: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        let b = app.new_master("", "Abertura", None).unwrap();
        // B on pages 2 (left-hand) and 3 (right-hand).
        app.apply_master(&pages[1..3], b);
        let rect = |app: &mut App, x: f64| {
            app.new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(x, 10.0, x + 20.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap()
        };
        // Into the master: its right-hand page first.
        app.edit_master(b);
        assert!(app.master_view.is_some());
        let right = rect(&mut app, 10.0);
        app.set_master_side(true);
        let left = rect(&mut app, 50.0);
        // On the master's layers, not on a page.
        let on_master = |app: &App, id| app.doc().master.iter().any(|l| l.shapes.iter().any(|s| s.id == id));
        assert!(on_master(&app, right) && on_master(&app, left));
        // While it is edited the page shows only the master.
        let shown: Vec<_> = app.page_layers().iter().flat_map(|l| l.shapes.iter().map(|s| s.id)).collect();
        assert!(shown.contains(&left) && !shown.contains(&right));
        // Back to the pages: page 2 (left) shows the left object, page 3
        // (right) the right one, page 1 (A) neither.
        app.goto_page(0);
        assert!(app.master_view.is_none());
        let on = |app: &App, page: PageId| -> Vec<tracedraw_core::ShapeId> {
            app.doc()
                .layers_for_page(page)
                .unwrap()
                .iter()
                .flat_map(|l| l.shapes.iter().map(|s| s.id))
                .collect()
        };
        assert!(on(&app, pages[1]).contains(&left) && !on(&app, pages[1]).contains(&right));
        assert!(on(&app, pages[2]).contains(&right) && !on(&app, pages[2]).contains(&left));
        assert!(!on(&app, pages[0]).contains(&left) && !on(&app, pages[0]).contains(&right));
    }

    #[test]
    fn prefixes_go_on_through_the_alphabet() {
        let mut ms = vec![MasterPage::a()];
        assert_eq!(free_prefix(&ms), "B");
        for c in 'B'..='Z' {
            ms.push(MasterPage {
                id: ms.len() as u32,
                prefix: c.to_string(),
                name: String::new(),
                based_on: None,
                color: None,
            });
        }
        assert_eq!(free_prefix(&ms), "AA");
    }

    #[test]
    fn a_page_saved_as_a_master_gives_it_its_objects() {
        let mut app = App::headless();
        let r = app
            .new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        let page = app.page;
        let m = app.page_as_master(page).expect("master");
        let layer = app
            .doc()
            .master
            .iter()
            .find(|l| l.master_page == m)
            .expect("its layer");
        assert_eq!(layer.shapes.len(), 1);
        assert_ne!(layer.shapes[0].id, r);
        // The page keeps its own object.
        assert!(app.doc().find_shape(r).is_some());
    }
}
