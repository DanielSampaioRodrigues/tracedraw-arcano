//! Arcano edition: named master pages (A-Master, B-Chapter opening) and
//! the page operations the Pages panel offers on them: new masters,
//! their options, applying a master to pages, pages that keep the master
//! of the page they come after. The masters themselves are
//! `tracedraw_core::document::MasterPage`; see docs/behavior/pages-panel.md.

use crate::app::App;
use crate::i18n::tr;
use tracedraw_core::document::{MasterPage, NO_MASTER};
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
    fn prefixes_go_on_through_the_alphabet() {
        let mut ms = vec![MasterPage::a()];
        assert_eq!(free_prefix(&ms), "B");
        for c in 'B'..='Z' {
            ms.push(MasterPage {
                id: ms.len() as u32,
                prefix: c.to_string(),
                name: String::new(),
                based_on: None,
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
