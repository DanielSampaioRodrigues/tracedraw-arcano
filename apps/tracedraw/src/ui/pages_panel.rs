//! Arcano edition: the Pages panel, as a layout program has it. Master
//! pages on top (None, A-Master, B-...), each with miniatures of its
//! left-hand and right-hand page; below them the drawing's pages as
//! miniatures, in spreads when the pages face each other, each with its
//! number and its master's prefix. Pages are picked (Shift for a run,
//! Ctrl for one more), dragged to move them, and a master is dragged onto
//! a page to apply it (onto the empty space under the pages: a new page
//! with it). A double click goes to a page; on a master it opens its
//! options. The panel's menu and the pages' right-click menu hold the
//! rest. See docs/behavior/pages-panel.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::masters::label;
use crate::page_thumbs::{self, Of};
use crate::theme::Tokens;
use crate::ui::tips::RichTip as _;
use egui::{pos2, vec2, Color32, Rect, Sense, Stroke, Ui};
use tracedraw_core::document::{MasterPage, PageNumberField, NO_MASTER};
use tracedraw_core::PageId;

/// Miniature heights by size setting, points.
const SIZES: [f32; 3] = [48.0, 72.0, 110.0];
/// The masters' miniatures, points.
const MASTER_H: f32 = 30.0;

/// Pages dragged in the panel.
#[derive(Clone, Debug)]
struct PageDrag(Vec<PageId>);
/// A master dragged from the panel's top ([`NO_MASTER`] for None).
#[derive(Clone, Copy, Debug)]
struct MasterDrag(u32);

/// One of the panel's small windows.
#[derive(Clone, Debug)]
enum Window {
    Insert {
        count: usize,
        /// 0 after the page, 1 before it, 2 at the start, 3 at the end.
        place: u8,
        page: usize,
        master: u32,
    },
    Move {
        pages: String,
        place: u8,
        page: usize,
    },
    Master {
        master: MasterPage,
        new: bool,
        /// Put the primary frame on the pages already using it too.
        fill: bool,
    },
    Apply {
        master: u32,
        pages: String,
    },
    /// Threaded text frames: pages (1-based), margins [top, bottom,
    /// inner, outer] mm, columns, gutter mm.
    Thread {
        from: usize,
        to: usize,
        margins: [f64; 4],
        columns: u32,
        gutter: f64,
    },
}

#[derive(Clone, Debug, Default)]
struct State {
    picked: Vec<PageId>,
    anchor: Option<PageId>,
    master: Option<u32>,
    window: Option<Window>,
}

fn state_id() -> egui::Id {
    egui::Id::new("arcano_pages_panel")
}

/// The pages `text` names ("1-4, 7"), as 0-based indices among `n`
/// pages, in order, each once; numbers out of range are left out.
pub fn parse_pages(text: &str, n: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for part in text.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<usize>(), b.trim().parse::<usize>()),
            None => (part.parse::<usize>(), part.parse::<usize>()),
        };
        if let (Ok(a), Ok(b)) = (a, b) {
            let (a, b) = (a.min(b), a.max(b));
            for i in a.max(1)..=b.min(n) {
                if !out.contains(&(i - 1)) {
                    out.push(i - 1);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// "1-3, 5" for the 0-based indices `idx` (sorted).
pub fn pages_text(idx: &[usize]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && idx[j + 1] == idx[j] + 1 {
            j += 1;
        }
        parts.push(if j > i {
            format!("{}-{}", idx[i] + 1, idx[j] + 1)
        } else {
            format!("{}", idx[i] + 1)
        });
        i = j + 1;
    }
    parts.join(", ")
}

/// The spreads of `n` pages: with facing pages page 1 alone on the right,
/// then pairs; else one page a row. Each slot is a page index or none.
pub fn spreads(n: usize, facing: bool) -> Vec<[Option<usize>; 2]> {
    if !facing {
        return (0..n).map(|i| [Some(i), None]).collect();
    }
    let mut out = Vec::new();
    if n > 0 {
        out.push([None, Some(0)]);
    }
    let mut i = 1;
    while i < n {
        out.push([Some(i), (i + 1 < n).then_some(i + 1)]);
        i += 2;
    }
    out
}

/// The number a page shows (its section's numbering).
fn page_number(app: &App, index: usize) -> String {
    let f = PageNumberField {
        start_at: 1,
        style: 0,
        prefix: String::new(),
        suffix: String::new(),
        show: 0,
    };
    app.doc().field_text(&f, index)
}

/// The panel.
pub fn pages_panel(app: &mut App, ui: &mut Ui) {
    if !app.has_document() {
        return;
    }
    let mut st: State = ui.data(|d| d.get_temp(state_id())).unwrap_or_default();
    let ids: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
    st.picked.retain(|p| ids.contains(p));
    let facing = app.doc().metadata.facing_pages;
    masters_part(app, ui, &mut st, facing);
    ui.separator();
    let footer_h = 38.0;
    let avail = (ui.available_height() - footer_h).max(80.0);
    egui::ScrollArea::vertical()
        .id_salt("arcano_pages_scroll")
        .max_height(avail)
        .auto_shrink([false, false])
        .show(ui, |ui| pages_part(app, ui, &mut st, facing));
    ui.separator();
    footer(app, ui, &mut st, facing);
    ui.data_mut(|d| d.insert_temp(state_id(), st));
}

/// Page > Threaded Text Frames: its window, for every page, with the
/// house margins.
pub fn open_thread_window(app: &App, ctx: &egui::Context) {
    let mut st: State = ctx.data(|d| d.get_temp(state_id())).unwrap_or_default();
    st.window = Some(thread_window(app));
    ctx.data_mut(|d| d.insert_temp(state_id(), st));
}

/// Threaded Text Frames as it opens: every page, the house margins.
fn thread_window(app: &App) -> Window {
    let h = crate::prelo_casa::house_prefs(app);
    let n = app.doc().pages.len().max(1);
    Window::Thread {
        from: 1,
        to: n,
        margins: [h.top, h.bottom, h.inner, h.outer],
        columns: 1,
        gutter: 5.0,
    }
}

/// The panel's small windows (Insert Pages, Master Options...), drawn
/// every frame whether the panel shows or not: a panel folded away by a
/// click in its window must not take the window with it.
pub fn pages_windows(app: &mut App, ctx: &egui::Context) {
    let mut st: State = ctx.data(|d| d.get_temp(state_id())).unwrap_or_default();
    if st.window.is_none() || !app.has_document() {
        return;
    }
    windows(app, ctx, &mut st);
    ctx.data_mut(|d| d.insert_temp(state_id(), st));
}

fn masters_part(app: &mut App, ui: &mut Ui, st: &mut State, facing: bool) {
    let masters = app.masters();
    let drawing = (app.engine.instance(), app.engine.revision());
    egui::ScrollArea::vertical()
        .id_salt("arcano_masters_scroll")
        .max_height(MASTER_H * 3.4 + 40.0)
        .show(ui, |ui| {
            // [None]
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click_and_drag());
            let resp = resp.rich_tip(crate::ui::tips::pair("pages.none", "pages.none_desc"));
            if resp.hovered() {
                ui.painter().rect_filled(r, 3.0, Tokens::tool_hover());
            }
            ui.painter().text(
                pos2(r.left() + 6.0, r.center().y),
                egui::Align2::LEFT_CENTER,
                tr("pages.none"),
                egui::FontId::proportional(12.0),
                Tokens::text_dim(),
            );
            if resp.drag_started() {
                resp.dnd_set_drag_payload(MasterDrag(NO_MASTER));
            }
            for m in &masters {
                let row_h = MASTER_H + 6.0;
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::hover());
                let resp = ui.interact(r, egui::Id::new(("arcano_master_row", m.id)), Sense::click_and_drag());
                let editing = app.master_view.is_some_and(|v| v.master == m.id);
                let picked = st.master == Some(m.id) || editing;
                if picked {
                    ui.painter().rect_filled(r, 3.0, Tokens::selected_row());
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 3.0, Tokens::tool_hover());
                }
                // Left and right miniatures (one without facing pages).
                let size = app.doc().pages.first().map(|p| p.size);
                let ratio = size.map_or(0.7, |s| (s.width / s.height.max(1e-6)) as f32);
                let w = MASTER_H * ratio;
                let mut x = r.left() + 6.0;
                let sides: &[bool] = if facing { &[true, false] } else { &[false] };
                for &left in sides {
                    let pr = Rect::from_min_size(pos2(x, r.top() + 3.0), vec2(w, MASTER_H));
                    paper(ui, pr);
                    let px = (MASTER_H * ui.ctx().pixels_per_point()) as u32;
                    if let Some(t) = page_thumbs::get(ui.ctx(), app.doc(), drawing, Of::Master(m.id, left), px) {
                        ui.painter().image(t, pr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    x += w;
                }
                // Its colour, as its pages' tags show it.
                let [cr, cg, cb] = m.label_color();
                ui.painter().rect_filled(
                    Rect::from_center_size(pos2(x + 9.0, r.center().y), vec2(8.0, 8.0)),
                    2.0,
                    Color32::from_rgb(cr, cg, cb),
                );
                let x = x + 10.0;
                ui.painter().text(
                    pos2(x + 8.0, r.center().y),
                    egui::Align2::LEFT_CENTER,
                    label(m),
                    egui::FontId::proportional(12.0),
                    Tokens::text(),
                );
                let based = m
                    .based_on
                    .and_then(|b| masters.iter().find(|x| x.id == b))
                    .map(|b| trf("pages.based_on_tip", &[("m", &label(b))]))
                    .unwrap_or_default();
                let resp = resp.rich_tip_with(|| {
                    format!("{}\n{}{}", label(m), tr("pages.master_desc"), if based.is_empty() { String::new() } else { format!("\n{based}") })
                });
                if resp.clicked() {
                    st.master = Some(m.id);
                }
                // As in a layout program: a double click opens the master
                // for editing.
                if resp.double_clicked() {
                    app.edit_master(m.id);
                }
                if resp.drag_started() {
                    resp.dnd_set_drag_payload(MasterDrag(m.id));
                }
                resp.context_menu(|ui| {
                    if ui.button(tr("pages.master_options")).clicked() {
                        st.window = Some(Window::Master {
                            master: m.clone(),
                            new: false,
                            fill: true,
                        });
                        ui.close();
                    }
                    if ui.button(tr("pages.apply_master")).clicked() {
                        st.window = Some(Window::Apply {
                            master: m.id,
                            pages: pages_text(&picked_indices(app, st)),
                        });
                        ui.close();
                    }
                    if ui.button(tr("pages.edit_master")).clicked() {
                        app.edit_master(m.id);
                        ui.close();
                    }
                    if ui
                        .add_enabled(m.id != 0, egui::Button::new(tr("pages.delete_master")))
                        .clicked()
                    {
                        app.delete_master(m.id);
                        ui.close();
                    }
                });
            }
        });
}

fn picked_indices(app: &App, st: &State) -> Vec<usize> {
    let mut v: Vec<usize> = app
        .doc()
        .pages
        .iter()
        .enumerate()
        .filter(|(_, p)| st.picked.contains(&p.id))
        .map(|(i, _)| i)
        .collect();
    if v.is_empty() {
        v.push(app.page_index());
    }
    v
}

/// White paper with a thin edge.
fn paper(ui: &Ui, r: Rect) {
    ui.painter().rect_filled(r, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::window_border()),
        egui::StrokeKind::Inside,
    );
}

fn pages_part(app: &mut App, ui: &mut Ui, st: &mut State, facing: bool) {
    let n = app.doc().pages.len();
    let h = SIZES[(app.settings.pages_thumb as usize).min(2)];
    let drawing = (app.engine.instance(), app.engine.revision());
    let rows = spreads(n, facing);
    let accent = Tokens::accent();
    let mut drop_at: Option<(usize, Vec<PageId>)> = None;
    let mut master_on: Option<(usize, u32)> = None;
    let mut go: Option<usize> = None;
    ui.add_space(6.0);
    let full = ui.available_width();
    for row in &rows {
        let widths: Vec<f32> = row
            .iter()
            .map(|s| {
                s.and_then(|i| app.doc().pages.get(i)).map_or(0.0, |p| {
                    h * (p.size.width / p.size.height.max(1e-6)) as f32
                })
            })
            .collect();
        // Facing: the spine in the middle of the panel.
        let left_w = if facing {
            widths[0].max(widths[1]).max(h * 0.7)
        } else {
            0.0
        };
        let (r, _) = ui.allocate_exact_size(vec2(full, h + 22.0), Sense::hover());
        let spine = if facing { r.center().x } else { r.center().x - widths[0] / 2.0 };
        for (side, slot) in row.iter().enumerate() {
            let Some(i) = *slot else { continue };
            let Some(page) = app.doc().pages.get(i).cloned() else { continue };
            let w = widths[side];
            let x0 = if facing {
                if side == 0 { spine - w } else { spine }
            } else {
                spine
            };
            let _ = left_w;
            let pr = Rect::from_min_size(pos2(x0, r.top() + 4.0), vec2(w, h));
            let picked = st.picked.contains(&page.id);
            if picked {
                ui.painter().rect_filled(pr.expand(3.0), 2.0, Tokens::selected_row());
            }
            paper(ui, pr);
            let px = (h * ui.ctx().pixels_per_point()) as u32;
            if let Some(t) = page_thumbs::get(ui.ctx(), app.doc(), drawing, Of::Page(page.id, i), px) {
                ui.painter().image(t, pr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            if page.id == app.page {
                ui.painter().rect_stroke(pr.expand(1.0), 0.0, Stroke::new(2.0, accent), egui::StrokeKind::Outside);
            }
            // The master's prefix in the outer top corner.
            // The master's prefix on a tag of its colour, in the outer top
            // corner.
            let m = app.doc().master_of(page.id);
            if m != NO_MASTER {
                if let Some(mp) = app.doc().master_page(m) {
                    let left_page = facing && side == 0;
                    let [r, g, b] = mp.label_color();
                    let font = egui::FontId::proportional(9.5);
                    let galley = ui.painter().layout_no_wrap(mp.prefix.clone(), font, Color32::WHITE);
                    let size = galley.size() + vec2(6.0, 2.0);
                    let at = if left_page {
                        pos2(pr.left() + 2.0, pr.top() + 2.0)
                    } else {
                        pos2(pr.right() - 2.0 - size.x, pr.top() + 2.0)
                    };
                    let tag = Rect::from_min_size(at, size);
                    ui.painter().rect_filled(tag, 2.0, Color32::from_rgb(r, g, b));
                    ui.painter().galley(tag.min + vec2(3.0, 1.0), galley, Color32::WHITE);
                }
            }
            ui.painter().text(
                pos2(pr.center().x, pr.bottom() + 3.0),
                egui::Align2::CENTER_TOP,
                page_number(app, i),
                egui::FontId::proportional(11.0),
                if page.id == app.page { Tokens::text() } else { Tokens::text_dim() },
            );
            let resp = ui.interact(pr, egui::Id::new(("arcano_page_thumb", page.id.raw())), Sense::click_and_drag());
            let resp = resp.rich_tip_with(|| {
                let mname = app
                    .doc()
                    .master_page(app.doc().master_of(page.id))
                    .map(|m| label(&m))
                    .unwrap_or_else(|| tr("pages.none"));
                format!("{}\n{}: {}\n{}", page.name, tr("pages.master"), mname, tr("pages.page_desc"))
            });
            if resp.clicked() {
                let mods = ui.input(|i| i.modifiers);
                if mods.command || mods.ctrl {
                    if let Some(k) = st.picked.iter().position(|p| *p == page.id) {
                        st.picked.remove(k);
                    } else {
                        st.picked.push(page.id);
                    }
                    st.anchor = Some(page.id);
                } else if mods.shift {
                    let a = st
                        .anchor
                        .and_then(|a| app.doc().pages.iter().position(|p| p.id == a))
                        .unwrap_or(i);
                    let (lo, hi) = (a.min(i), a.max(i));
                    st.picked = app.doc().pages[lo..=hi].iter().map(|p| p.id).collect();
                } else {
                    st.picked = vec![page.id];
                    st.anchor = Some(page.id);
                }
            }
            if resp.double_clicked() {
                go = Some(i);
            }
            if resp.drag_started() {
                if !st.picked.contains(&page.id) {
                    st.picked = vec![page.id];
                    st.anchor = Some(page.id);
                }
                resp.dnd_set_drag_payload(PageDrag(st.picked.clone()));
            }
            // A master over the page: applied to it (to the picked pages
            // when it is one of them).
            if let Some(md) = resp.dnd_hover_payload::<MasterDrag>() {
                ui.painter().rect_stroke(pr.expand(2.0), 0.0, Stroke::new(2.0, accent), egui::StrokeKind::Outside);
                let _ = md;
            }
            if let Some(md) = resp.dnd_release_payload::<MasterDrag>() {
                master_on = Some((i, md.0));
            }
            // Pages over the page: they go before or after it.
            if resp.dnd_hover_payload::<PageDrag>().is_some() {
                let pointer = ui.ctx().pointer_hover_pos().unwrap_or(pr.center());
                let after = pointer.x > pr.center().x;
                let x = if after { pr.right() + 2.0 } else { pr.left() - 2.0 };
                ui.painter().line_segment(
                    [pos2(x, pr.top() - 2.0), pos2(x, pr.bottom() + 2.0)],
                    Stroke::new(3.0, accent),
                );
            }
            if let Some(pd) = resp.dnd_release_payload::<PageDrag>() {
                let pointer = ui.ctx().pointer_interact_pos().unwrap_or(pr.center());
                let at = if pointer.x > pr.center().x { i + 1 } else { i };
                drop_at = Some((at, pd.0.clone()));
            }
            let picked_now = picked_indices(app, st);
            resp.context_menu(|ui| page_menu(app, ui, st, &picked_now, facing));
        }
    }
    // The space under the pages: a master there makes a page with it,
    // pages dropped there go to the end.
    let rest_h = ui.available_height().max(40.0);
    let (rest, rresp) = ui.allocate_exact_size(vec2(full, rest_h), Sense::click());
    if rresp.dnd_hover_payload::<MasterDrag>().is_some() || rresp.dnd_hover_payload::<PageDrag>().is_some() {
        ui.painter().line_segment(
            [pos2(rest.left() + 20.0, rest.top() + 4.0), pos2(rest.right() - 20.0, rest.top() + 4.0)],
            Stroke::new(3.0, accent),
        );
    }
    if let Some(md) = rresp.dnd_release_payload::<MasterDrag>() {
        let n = app.doc().pages.len();
        app.insert_pages(1, n, md.0);
    }
    if let Some(pd) = rresp.dnd_release_payload::<PageDrag>() {
        drop_at = Some((n, pd.0.clone()));
    }
    if rresp.clicked() {
        st.picked.clear();
    }
    let picked_now = picked_indices(app, st);
    rresp.context_menu(|ui| page_menu(app, ui, st, &picked_now, facing));
    if let Some((i, m)) = master_on {
        let Some(target) = app.doc().pages.get(i).map(|p| p.id) else { return };
        let pages = if st.picked.contains(&target) { st.picked.clone() } else { vec![target] };
        app.apply_master(&pages, m);
    }
    if let Some((at, moving)) = drop_at {
        // `at` counts every page; the place among those not moved.
        let order: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        let to = order[..at.min(order.len())]
            .iter()
            .filter(|p| !moving.contains(p))
            .count();
        app.move_pages(&moving, to);
    }
    if let Some(i) = go {
        app.goto_page(i);
    }
}

/// The right-click menu of the pages and the panel's menu share these.
fn page_menu(app: &mut App, ui: &mut Ui, st: &mut State, picked: &[usize], facing: bool) {
    let n = app.doc().pages.len();
    let first = picked.first().copied().unwrap_or(0);
    if ui.button(tr("pages.insert")).clicked() {
        st.window = Some(Window::Insert {
            count: 1,
            place: 0,
            page: first + 1,
            master: app.doc().pages.get(first).map_or(0, |p| app.doc().master_of(p.id)),
        });
        ui.close();
    }
    if ui.button(tr("pages.move")).clicked() {
        st.window = Some(Window::Move {
            pages: pages_text(picked),
            place: 0,
            page: n,
        });
        ui.close();
    }
    if ui.button(tr("pages.duplicate")).clicked() {
        duplicate_picked(app, picked);
        ui.close();
    }
    if ui
        .add_enabled(picked.len() < n, egui::Button::new(tr("pages.delete")))
        .clicked()
    {
        let ids: Vec<PageId> = picked.iter().filter_map(|i| app.doc().pages.get(*i).map(|p| p.id)).collect();
        app.delete_pages_by_id(&ids);
        st.picked.clear();
        ui.close();
    }
    ui.separator();
    if ui.button(tr("pages.apply_master")).clicked() {
        st.window = Some(Window::Apply {
            master: st.master.unwrap_or(0),
            pages: pages_text(picked),
        });
        ui.close();
    }
    if ui.button(tr("pages.save_as_master")).clicked() {
        if let Some(p) = app.doc().pages.get(first).map(|p| p.id) {
            if let Some(m) = app.page_as_master(p) {
                st.master = Some(m);
            }
        }
        ui.close();
    }
    if ui.button(tr("pages.new_master")).clicked() {
        let masters = app.masters();
        st.window = Some(Window::Master {
            master: MasterPage {
                id: u32::MAX - 1,
                prefix: crate::masters::free_prefix(&masters),
                name: tr("master.default_name"),
                based_on: None,
                color: None,
                primary: None,
            },
            new: true,
            fill: true,
        });
        ui.close();
    }
    if ui.button(tr("pages.thread")).clicked() {
        let h = crate::prelo_casa::house_prefs(app);
        let (a, b) = (picked.first().copied().unwrap_or(0), picked.last().copied().unwrap_or(n.saturating_sub(1)));
        let (a, b) = if a == b { (0, n.saturating_sub(1)) } else { (a, b) };
        st.window = Some(Window::Thread {
            from: a + 1,
            to: b + 1,
            margins: [h.top, h.bottom, h.inner, h.outer],
            columns: 1,
            gutter: 5.0,
        });
        ui.close();
    }
    ui.separator();
    if ui.button(tr("pages.sections")).clicked() {
        if let Some(p) = app.doc().pages.get(first).map(|p| p.id) {
            if app.page != p {
                app.goto_page(first);
            }
        }
        app.dialog = crate::ui::dialogs::Dialog::Sections;
        ui.close();
    }
    let mut f = facing;
    if ui.checkbox(&mut f, tr("menu.layout.facing_pages")).changed() {
        let start = app.doc().metadata.facing_start;
        app.set_facing_pages(f, start);
        ui.close();
    }
    ui.menu_button(tr("pages.thumb_size"), |ui| {
        for (k, key) in ["pages.size_small", "pages.size_medium", "pages.size_large"].iter().enumerate() {
            if ui.radio(app.settings.pages_thumb as usize == k, tr(key)).clicked() {
                app.settings.pages_thumb = k as u8;
                ui.close();
            }
        }
    });
}

/// Duplicate the pages `picked` (indices), each copy after the last of
/// them, in one undo step.
fn duplicate_picked(app: &mut App, picked: &[usize]) {
    let ids: Vec<PageId> = picked.iter().filter_map(|i| app.doc().pages.get(*i).map(|p| p.id)).collect();
    if ids.is_empty() {
        return;
    }
    let mark = app.engine.begin_group();
    let mut copies = Vec::new();
    for id in &ids {
        let before: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        app.run(tracedraw_core::Command::DuplicatePage {
            page: *id,
            before: false,
            contents: true,
        });
        if let Some(c) = app.doc().pages.iter().map(|p| p.id).find(|p| !before.contains(p)) {
            app.inherit_master(c, *id);
            copies.push(c);
        }
    }
    // The copies together after the last page duplicated.
    let last = ids.last().and_then(|l| app.doc().pages.iter().position(|p| p.id == *l)).unwrap_or(0);
    let order: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
    let to = order[..=last.min(order.len().saturating_sub(1))]
        .iter()
        .filter(|p| !copies.contains(p))
        .count();
    app.engine.end_group(mark, "Duplicate Page");
    app.move_pages(&copies, to);
}

fn footer(app: &mut App, ui: &mut Ui, st: &mut State, facing: bool) {
    let n = app.doc().pages.len();
    ui.horizontal(|ui| {
        let text = if facing {
            trf(
                "pages.count_spreads",
                &[("n", &n.to_string()), ("s", &spreads(n, true).len().to_string())],
            )
        } else {
            trf("pages.count", &[("n", &n.to_string())])
        };
        let picked = picked_indices(app, st);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let more = icon_button(ui, true, paint_dots)
                .rich_tip(crate::ui::tips::pair("pages.menu", "pages.menu_desc"));
            egui::Popup::menu(&more).show(|ui| page_menu(app, ui, st, &picked, facing));
            let del = icon_button(ui, picked.len() < n, paint_trash)
                .rich_tip(crate::ui::tips::pair("pages.delete", "pages.delete_desc"));
            if del.clicked() {
                let ids: Vec<PageId> = picked.iter().filter_map(|i| app.doc().pages.get(*i).map(|p| p.id)).collect();
                app.delete_pages_by_id(&ids);
                st.picked.clear();
            }
            // Threaded text frames at hand, as in the Page menu.
            let thread = icon_button(ui, true, paint_threaded_frames).rich_tip(
                crate::ui::tips::pair("menu.layout.thread_frames", "menu.layout.thread_frames_desc"),
            );
            #[cfg(test)]
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pages_thread_button"), thread.rect));
            if thread.clicked() {
                // The panel keeps its state: the window goes in it.
                st.window = Some(thread_window(app));
            }
            let add = icon_button(ui, true, paint_new_page)
                .rich_tip(crate::ui::tips::pair("pages.new_page", "pages.new_page_desc"));
            if add.clicked() {
                let at = picked.last().copied().unwrap_or(app.page_index());
                let m = app.doc().pages.get(at).map_or(0, |p| app.doc().master_of(p.id));
                app.insert_pages(1, at + 1, m);
            }
            ui.add(egui::Label::new(egui::RichText::new(text).color(Tokens::text_dim()).size(11.0)).truncate());
        });
    });
}

/// A small button with a picture drawn by `paint`.
fn icon_button(ui: &mut Ui, enabled: bool, paint: fn(&egui::Painter, Rect, Color32)) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 3.0, Tokens::tool_hover());
    }
    let c = if enabled { Tokens::icon() } else { Tokens::text_dim() };
    paint(ui.painter(), r.shrink(4.0), c);
    resp
}

/// A page with a plus beside its corner.
fn paint_new_page(p: &egui::Painter, r: Rect, c: Color32) {
    let s = Stroke::new(1.2, c);
    let page = Rect::from_min_max(r.min, pos2(r.left() + r.width() * 0.62, r.bottom() - 2.0));
    p.rect_stroke(page, 0.0, s, egui::StrokeKind::Inside);
    let at = pos2(r.right() - 3.0, r.bottom() - 3.5);
    p.line_segment([at - vec2(3.5, 0.0), at + vec2(3.5, 0.0)], Stroke::new(1.6, c));
    p.line_segment([at - vec2(0.0, 3.5), at + vec2(0.0, 3.5)], Stroke::new(1.6, c));
}

/// Two text frames, the first's out port linked to the second's in
/// port.
fn paint_threaded_frames(p: &egui::Painter, r: Rect, c: Color32) {
    let s = Stroke::new(1.2, c);
    let w = r.width() * 0.42;
    let a = Rect::from_min_size(r.min, vec2(w, r.height() * 0.7));
    let b = Rect::from_min_size(pos2(r.right() - w, r.top() + r.height() * 0.3), vec2(w, r.height() * 0.7));
    p.rect_stroke(a, 0.0, s, egui::StrokeKind::Inside);
    p.rect_stroke(b, 0.0, s, egui::StrokeKind::Inside);
    for k in [0.3, 0.5] {
        let y = a.top() + a.height() * k;
        p.line_segment([pos2(a.left() + 1.5, y), pos2(a.right() - 1.5, y)], Stroke::new(0.8, c));
    }
    p.line_segment([pos2(a.right(), a.bottom() - 1.5), pos2(b.left(), b.top() + 1.5)], Stroke::new(1.2, c));
}

/// Three dots: the panel's menu.
fn paint_dots(p: &egui::Painter, r: Rect, c: Color32) {
    for k in [-4.0, 0.0, 4.0] {
        p.circle_filled(r.center() + vec2(k, 0.0), 1.4, c);
    }
}

/// A bin.
fn paint_trash(p: &egui::Painter, r: Rect, c: Color32) {
    let s = Stroke::new(1.2, c);
    let top = r.top() + 3.0;
    p.line_segment([pos2(r.left(), top), pos2(r.right(), top)], s);
    p.line_segment([pos2(r.center().x - 2.0, r.top() + 1.0), pos2(r.center().x + 2.0, r.top() + 1.0)], s);
    let body = [
        pos2(r.left() + 1.5, top),
        pos2(r.left() + 2.5, r.bottom()),
        pos2(r.right() - 2.5, r.bottom()),
        pos2(r.right() - 1.5, top),
    ];
    p.line_segment([body[0], body[1]], s);
    p.line_segment([body[1], body[2]], s);
    p.line_segment([body[2], body[3]], s);
    for k in [-2.0, 0.0, 2.0] {
        let x = r.center().x + k;
        p.line_segment([pos2(x, top + 2.5), pos2(x, r.bottom() - 2.0)], Stroke::new(1.0, c));
    }
}

fn windows(app: &mut App, ctx: &egui::Context, st: &mut State) {
    let Some(mut w) = st.window.take() else { return };
    let mut close = false;
    let n = app.doc().pages.len();
    let masters = app.masters();
    let master_combo = |ui: &mut Ui, id: &str, value: &mut u32, with_none: bool| {
        let shown = if *value == NO_MASTER {
            tr("pages.none")
        } else {
            masters.iter().find(|m| m.id == *value).map(label).unwrap_or_default()
        };
        crate::ui::combo::from_id_salt(id).selected_text(shown).show_ui(ui, |ui| {
            if with_none {
                ui.selectable_value(value, NO_MASTER, tr("pages.none"));
            }
            for m in &masters {
                ui.selectable_value(value, m.id, label(m));
            }
        });
    };
    let places = ["pages.after_page", "pages.before_page", "pages.at_start", "pages.at_end"];
    match &mut w {
        Window::Insert { count, place, page, master } => {
            crate::ui::dialogs::window(ctx, tr("pages.insert_title")).show(ctx, |ui| {
                egui::Grid::new("arcano_insert_pages").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("pages.how_many"));
                    ui.add(egui::DragValue::new(count).range(1..=9999).speed(0.2));
                    ui.end_row();
                    ui.label(tr("pages.where"));
                    ui.horizontal(|ui| {
                        crate::ui::combo::from_id_salt("arcano_insert_place")
                            .selected_text(tr(places[*place as usize % 4]))
                            .show_ui(ui, |ui| {
                                for (k, key) in places.iter().enumerate() {
                                    ui.selectable_value(place, k as u8, tr(key));
                                }
                            });
                        if *place < 2 {
                            ui.add(egui::DragValue::new(page).range(1..=n.max(1)).speed(0.2));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("pages.master"));
                    master_combo(ui, "arcano_insert_master", master, true);
                    ui.end_row();
                });
                ui.separator();
                if crate::ui::dialogs::ok_cancel(ui, &mut close) {
                    let at = match *place {
                        0 => (*page).min(n),
                        1 => page.saturating_sub(1).min(n),
                        2 => 0,
                        _ => n,
                    };
                    app.insert_pages(*count, at, *master);
                    close = true;
                }
            });
        }
        Window::Move { pages, place, page } => {
            crate::ui::dialogs::window(ctx, tr("pages.move_title")).show(ctx, |ui| {
                egui::Grid::new("arcano_move_pages").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("pages.which"));
                    ui.text_edit_singleline(pages)
                        .rich_tip(tr("pages.which_desc"));
                    ui.end_row();
                    ui.label(tr("pages.where"));
                    ui.horizontal(|ui| {
                        crate::ui::combo::from_id_salt("arcano_move_place")
                            .selected_text(tr(places[*place as usize % 4]))
                            .show_ui(ui, |ui| {
                                for (k, key) in places.iter().enumerate() {
                                    ui.selectable_value(place, k as u8, tr(key));
                                }
                            });
                        if *place < 2 {
                            ui.add(egui::DragValue::new(page).range(1..=n.max(1)).speed(0.2));
                        }
                    });
                    ui.end_row();
                });
                ui.separator();
                if crate::ui::dialogs::ok_cancel(ui, &mut close) {
                    let idx = parse_pages(pages, n);
                    let ids: Vec<PageId> = idx.iter().filter_map(|i| app.doc().pages.get(*i).map(|p| p.id)).collect();
                    let at = match *place {
                        0 => (*page).min(n),
                        1 => page.saturating_sub(1).min(n),
                        2 => 0,
                        _ => n,
                    };
                    let order: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
                    let to = order[..at.min(order.len())].iter().filter(|p| !ids.contains(p)).count();
                    app.move_pages(&ids, to);
                    close = true;
                }
            });
        }
        Window::Master { master, new, fill } => {
            let title = if *new { tr("pages.new_master") } else { tr("pages.master_options") };
            crate::ui::dialogs::window(ctx, title).show(ctx, |ui| {
                egui::Grid::new("arcano_master_options").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("pages.prefix"));
                    ui.add(egui::TextEdit::singleline(&mut master.prefix).desired_width(50.0).char_limit(4));
                    ui.end_row();
                    ui.label(tr("pages.name"));
                    ui.text_edit_singleline(&mut master.name);
                    ui.end_row();
                    ui.label(tr("pages.based_on"));
                    let mut b = master.based_on.unwrap_or(NO_MASTER);
                    let me = master.id;
                    let shown = if b == NO_MASTER {
                        tr("pages.none")
                    } else {
                        masters.iter().find(|m| m.id == b).map(label).unwrap_or_default()
                    };
                    crate::ui::combo::from_id_salt("arcano_master_based_on").selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut b, NO_MASTER, tr("pages.none"));
                        for m in masters.iter().filter(|m| m.id != me && !app.doc().master_would_loop(me, Some(m.id))) {
                            ui.selectable_value(&mut b, m.id, label(m));
                        }
                    });
                    master.based_on = (b != NO_MASTER).then_some(b);
                    ui.end_row();
                    // The colour marking its pages.
                    ui.label(tr("pages.color"));
                    ui.horizontal_wrapped(|ui| {
                        let now = master.label_color();
                        for c in MasterPage::LABEL_COLORS {
                            let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                            ui.painter().rect_filled(r, 3.0, Color32::from_rgb(c[0], c[1], c[2]));
                            if c == now {
                                ui.painter().rect_stroke(r.expand(1.5), 4.0, Stroke::new(2.0, Tokens::text()), egui::StrokeKind::Outside);
                            }
                            if resp.rich_tip(tr("pages.color_desc")).clicked() {
                                master.color = Some(c);
                            }
                        }
                        let mut rgb = now;
                        if ui
                            .color_edit_button_srgb(&mut rgb)
                            .rich_tip(crate::ui::tips::pair("pages.color_free", "pages.color_desc"))
                            .changed()
                        {
                            master.color = Some(rgb);
                        }
                    });
                    ui.end_row();
                    // The primary text frame its pages get.
                    ui.label(tr("pages.primary"));
                    let mut on = master.primary.is_some();
                    if ui
                        .checkbox(&mut on, tr("pages.primary_on"))
                        .rich_tip(crate::ui::tips::pair("pages.primary", "pages.primary_desc"))
                        .changed()
                    {
                        master.primary = on.then(|| {
                            let h = crate::prelo_casa::house_prefs(app);
                            tracedraw_core::PrimaryFrame {
                                margins: [h.top, h.bottom, h.inner, h.outer],
                                columns: 1,
                                gutter: 5.0,
                            }
                        });
                    }
                    ui.end_row();
                    if let Some(p) = master.primary.as_mut() {
                        for (k, key) in ["pages.margin_top", "pages.margin_bottom", "pages.margin_inner", "pages.margin_outer"].iter().enumerate() {
                            ui.label(tr(key));
                            ui.add(egui::DragValue::new(&mut p.margins[k]).range(0.0..=500.0).speed(0.5).suffix(" mm"))
                                .rich_tip(crate::ui::tips::pair(key, "pages.margin_desc"));
                            ui.end_row();
                        }
                        ui.label(tr("pages.columns"));
                        ui.add(egui::DragValue::new(&mut p.columns).range(1..=20).speed(0.1))
                            .rich_tip(tr("pages.columns"));
                        ui.end_row();
                        ui.label(tr("pages.gutter"));
                        ui.add(egui::DragValue::new(&mut p.gutter).range(0.0..=100.0).speed(0.2).suffix(" mm"))
                            .rich_tip(tr("pages.gutter"));
                        ui.end_row();
                        if !*new {
                            ui.label("");
                            ui.checkbox(fill, tr("pages.primary_fill"))
                                .rich_tip(crate::ui::tips::pair("pages.primary_fill", "pages.primary_fill_desc"));
                            ui.end_row();
                        }
                    }
                });
                ui.separator();
                if crate::ui::dialogs::ok_cancel(ui, &mut close) {
                    close = app.master_options_ok(master.clone(), *new, *fill).inspect(|id| {
                        if *new {
                            st.master = Some(*id);
                        }
                    }).is_some();
                }
            });
        }
        Window::Thread { from, to, margins, columns, gutter } => {
            crate::ui::dialogs::window(ctx, tr("pages.thread_title")).show(ctx, |ui| {
                ui.label(egui::RichText::new(tr("pages.thread_intro")).color(Tokens::text_dim()));
                ui.add_space(4.0);
                egui::Grid::new("arcano_thread_frames").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("pages.thread_pages"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(from).range(1..=n.max(1)).speed(0.2));
                        ui.label(tr("pages.thread_to"));
                        ui.add(egui::DragValue::new(to).range(1..=n.max(1)).speed(0.2));
                    });
                    ui.end_row();
                    for (k, key) in ["pages.margin_top", "pages.margin_bottom", "pages.margin_inner", "pages.margin_outer"].iter().enumerate() {
                        ui.label(tr(key));
                        ui.add(egui::DragValue::new(&mut margins[k]).range(0.0..=500.0).speed(0.5).suffix(" mm"))
                            .rich_tip(crate::ui::tips::pair(key, "pages.margin_desc"));
                        ui.end_row();
                    }
                    ui.label(tr("pages.columns"));
                    ui.add(egui::DragValue::new(columns).range(1..=20).speed(0.1));
                    ui.end_row();
                    ui.label(tr("pages.gutter"));
                    ui.add(egui::DragValue::new(gutter).range(0.0..=100.0).speed(0.2).suffix(" mm"));
                    ui.end_row();
                });
                ui.separator();
                if crate::ui::dialogs::ok_cancel(ui, &mut close) {
                    let (a, b) = ((*from).min(*to), (*from).max(*to));
                    app.thread_page_frames(a.saturating_sub(1), b.saturating_sub(1), *margins, *columns, *gutter);
                    close = true;
                }
            });
        }
        Window::Apply { master, pages } => {
            crate::ui::dialogs::window(ctx, tr("pages.apply_title")).show(ctx, |ui| {
                egui::Grid::new("arcano_apply_master").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("pages.master"));
                    master_combo(ui, "arcano_apply_master_combo", master, true);
                    ui.end_row();
                    ui.label(tr("pages.to_pages"));
                    ui.text_edit_singleline(pages).rich_tip(tr("pages.which_desc"));
                    ui.end_row();
                });
                ui.separator();
                if crate::ui::dialogs::ok_cancel(ui, &mut close) {
                    let idx = parse_pages(pages, n);
                    let ids: Vec<PageId> = idx.iter().filter_map(|i| app.doc().pages.get(*i).map(|p| p.id)).collect();
                    app.apply_master(&ids, *master);
                    close = true;
                }
            });
        }
    }
    if !close {
        st.window = Some(w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_lists_read_and_write_runs() {
        assert_eq!(parse_pages("1-3, 5", 10), vec![0, 1, 2, 4]);
        assert_eq!(parse_pages("5-3; 3, 40", 10), vec![2, 3, 4]);
        assert_eq!(parse_pages("x, 0, 2", 3), vec![1]);
        assert_eq!(pages_text(&[0, 1, 2, 4]), "1-3, 5");
        assert_eq!(pages_text(&[6]), "7");
    }

    #[test]
    fn facing_spreads_start_with_page_one_alone() {
        assert_eq!(spreads(4, true), vec![[None, Some(0)], [Some(1), Some(2)], [Some(3), None]]);
        assert_eq!(spreads(2, false), vec![[Some(0), None], [Some(1), None]]);
        assert!(spreads(0, true).is_empty());
    }

    #[test]
    fn the_panel_draws_and_a_master_dropped_on_a_page_applies_it() {
        let ctx = crate::theme::ui_context();
        App::setup_context(&ctx);
        let mut app = App::headless();
        app.add_page();
        app.add_page();
        let b = app.new_master("", "Abertura", None).unwrap();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 700.0));
        let mut frame = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| pages_panel(app, ui));
            out.textures_delta.clear();
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        // The page miniatures have their places.
        let place = |id: PageId| {
            ctx.read_response(egui::Id::new(("arcano_page_thumb", id.raw())))
                .map(|r| r.rect)
                .expect("the page's miniature")
        };
        let pages: Vec<PageId> = app.doc().pages.iter().map(|p| p.id).collect();
        let p2 = place(pages[1]);
        let b_row = ctx
            .read_response(egui::Id::new(("arcano_master_row", b)))
            .map(|r| r.rect.center())
            .expect("B's row");
        let press = |p, down| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![egui::Event::PointerMoved(b_row)]);
        frame(&mut app, vec![press(b_row, true)]);
        for k in 1..=8 {
            let t = k as f32 / 8.0;
            let p = b_row + (p2.center() - b_row) * t;
            frame(&mut app, vec![egui::Event::PointerMoved(p)]);
        }
        frame(&mut app, vec![press(p2.center(), false)]);
        frame(&mut app, vec![]);
        assert_eq!(app.doc().master_of(pages[1]), b);
        assert_eq!(app.doc().master_of(pages[0]), 0);
    }

    /// The button with two linked frames at the foot of the panel opens
    /// Threaded Text Frames, as the Page menu does.
    #[test]
    fn the_foot_of_the_panel_opens_threaded_text_frames() {
        let ctx = crate::theme::ui_context();
        App::setup_context(&ctx);
        let mut app = App::headless();
        app.add_page();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 700.0));
        let frame = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| pages_panel(app, ui));
            out.textures_delta.clear();
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        let at = ctx
            .data(|d| d.get_temp::<Rect>(egui::Id::new("pages_thread_button")))
            .expect("the button")
            .center();
        let press = |down| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![egui::Event::PointerMoved(at)]);
        frame(&mut app, vec![press(true)]);
        frame(&mut app, vec![press(false)]);
        let st: State = ctx.data(|d| d.get_temp(state_id())).unwrap_or_default();
        assert!(matches!(st.window, Some(Window::Thread { from: 1, to: 2, .. })));
    }
}
