//! Stacked panels (Options > Customization > Appearance > Stacked
//! panels, on by default): a docked column shows several panel groups at
//! once, one under the other, each with its tabs across its top and a
//! divider under it to share the height. A tab dropped on a group's tabs
//! joins that group; dropped on the lower half of a group it starts a
//! group of its own under it. A collapsed column is a strip of the
//! panels' pictures: a click opens the column with that panel in front.
//!
//! The groups only divide the column's panels: the column keeps its list
//! (`App::docker_column`), so opening, closing and moving panels work as
//! before; each frame the groups follow it (a panel that left goes, one
//! that came joins the group of the panel in front). See
//! docs/behavior/stacked-panels.md.

use crate::app::{App, DockerDrag, DockerPlace, DockerTab};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip as _;
use egui::{pos2, vec2, Rect, Sense, Ui};

/// Height of a group's tab row.
pub const HEAD_H: f32 = 30.0;
/// Height of the divider between groups.
const SPLIT_H: f32 = 6.0;
/// The least height a group keeps.
const MIN_GROUP_H: f32 = 90.0;
/// Width of a collapsed column's strip of pictures.
pub const STRIP_W: f32 = 40.0;

/// One group of a column: its panels, the one in front and its share of
/// the column's height.
#[derive(Debug, Clone, PartialEq)]
pub struct StackGroup {
    pub tabs: Vec<DockerTab>,
    pub front: DockerTab,
    pub weight: f32,
}

/// The groups of the four docked columns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stacks {
    pub columns: [Vec<StackGroup>; 4],
}

fn index(place: DockerPlace) -> Option<usize> {
    match place {
        DockerPlace::Right => Some(0),
        DockerPlace::Left => Some(1),
        DockerPlace::RightInner => Some(2),
        DockerPlace::LeftInner => Some(3),
        DockerPlace::Floating => None,
    }
}

impl Stacks {
    pub fn groups(&self, place: DockerPlace) -> &[StackGroup] {
        index(place).map_or(&[], |i| &self.columns[i])
    }

    /// The groups made to follow the column's panels `tabs` with `front`
    /// in front of its group.
    pub fn follow(&mut self, place: DockerPlace, tabs: &[DockerTab], front: DockerTab) {
        let Some(i) = index(place) else { return };
        let groups = &mut self.columns[i];
        for g in groups.iter_mut() {
            g.tabs.retain(|t| tabs.contains(t));
        }
        groups.retain(|g| !g.tabs.is_empty());
        for t in tabs {
            if groups.iter().any(|g| g.tabs.contains(t)) {
                continue;
            }
            // A panel that came joins the group in front's, else the first.
            let at = groups
                .iter()
                .position(|g| g.tabs.contains(&front))
                .unwrap_or(0);
            match groups.get_mut(at) {
                Some(g) => {
                    g.tabs.push(*t);
                    g.front = *t;
                }
                None => groups.push(StackGroup {
                    tabs: vec![*t],
                    front: *t,
                    weight: 1.0,
                }),
            }
        }
        for g in groups.iter_mut() {
            if g.tabs.contains(&front) {
                g.front = front;
            }
            if !g.tabs.contains(&g.front) {
                g.front = g.tabs[0];
            }
            if !(g.weight.is_finite() && g.weight > 0.0) {
                g.weight = 1.0;
            }
        }
    }

    /// Put `tab` (already in the column) in group `group`, at its end, in
    /// front; `new_after`: in a new group of its own after `group`.
    pub fn regroup(&mut self, place: DockerPlace, tab: DockerTab, group: usize, new_after: bool) {
        let Some(i) = index(place) else { return };
        let groups = &mut self.columns[i];
        // The group it goes to, counted before it leaves its own.
        let target = groups.get(group).map(|g| g.tabs.clone());
        for g in groups.iter_mut() {
            g.tabs.retain(|t| *t != tab);
        }
        let mut at = target
            .and_then(|tabs| {
                groups
                    .iter()
                    .position(|g| g.tabs.iter().any(|t| tabs.contains(t)))
            })
            .unwrap_or(groups.len().saturating_sub(1));
        let emptied_before = groups.iter().take(at).filter(|g| g.tabs.is_empty()).count();
        groups.retain(|g| !g.tabs.is_empty());
        at = at.saturating_sub(emptied_before);
        if new_after || groups.is_empty() {
            let pos = (at + 1).min(groups.len());
            groups.insert(
                pos,
                StackGroup {
                    tabs: vec![tab],
                    front: tab,
                    weight: 1.0,
                },
            );
        } else if let Some(g) = groups.get_mut(at) {
            g.tabs.push(tab);
            g.front = tab;
        }
    }
}

/// Where a dragged tab would go over the stacked column at `place` (laid
/// out this frame or the last): into group `group`, or a new group after
/// it, and the band to show.
#[derive(Debug, Clone, Copy)]
struct StackLayout {
    frame: u64,
    heads: [Option<Rect>; 8],
    bodies: [Option<Rect>; 8],
}

fn layout_id(place: DockerPlace) -> egui::Id {
    egui::Id::new(("panel_stack_layout", place as u8))
}

/// A tab dragged with the pointer at `p` over a stacked column: the
/// column, the group, whether it starts a new group after it, and the
/// band that shows where it goes.
pub fn drop_on(ctx: &egui::Context, p: egui::Pos2) -> Option<(DockerPlace, usize, bool, Rect)> {
    let nr = ctx.cumulative_frame_nr();
    [
        DockerPlace::Right,
        DockerPlace::RightInner,
        DockerPlace::Left,
        DockerPlace::LeftInner,
    ]
    .into_iter()
    .find_map(|place| {
        let l = ctx.data(|d| d.get_temp::<StackLayout>(layout_id(place)))?;
        if l.frame + 1 < nr {
            return None;
        }
        for g in 0..8 {
            if let Some(h) = l.heads[g] {
                if h.contains(p) {
                    return Some((place, g, false, h));
                }
            }
            if let Some(b) = l.bodies[g] {
                if b.contains(p) {
                    // The upper half joins the group, the lower half
                    // starts a group under it.
                    let lower = p.y > b.center().y;
                    let band = if lower {
                        Rect::from_min_max(pos2(b.left(), b.center().y), b.max)
                    } else {
                        l.heads[g].map_or(b, |h| {
                            h.union(Rect::from_min_max(b.min, pos2(b.right(), b.center().y)))
                        })
                    };
                    return Some((place, g, lower, band));
                }
            }
        }
        None
    })
}

/// The stacked column at `place`, open: its groups one under the other.
pub fn column(app: &mut App, ui: &mut Ui, place: DockerPlace) {
    let (tabs, front, _) = app.docker_column(place);
    let tabs = tabs.to_vec();
    app.stacks.follow(place, &tabs, front);
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, Tokens::panel());
    let groups = app.stacks.groups(place).to_vec();
    let n = groups.len().max(1);
    let room = (full.height() - SPLIT_H * (n as f32 - 1.0)).max(0.0);
    let total: f32 = groups.iter().map(|g| g.weight).sum::<f32>().max(0.001);
    let mut layout = StackLayout {
        frame: ui.ctx().cumulative_frame_nr(),
        heads: [None; 8],
        bodies: [None; 8],
    };
    let mut y = full.top();
    for (i, g) in groups.iter().enumerate() {
        let h = if i + 1 == n {
            full.bottom() - y
        } else {
            (room * g.weight / total).max(MIN_GROUP_H.min(room / n as f32))
        };
        let rect = Rect::from_min_max(
            pos2(full.left(), y),
            pos2(full.right(), (y + h).min(full.bottom())),
        );
        let head = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + HEAD_H));
        let body = Rect::from_min_max(pos2(rect.left(), head.bottom()), rect.max);
        if i < 8 {
            layout.heads[i] = Some(head);
            layout.bodies[i] = Some(body);
        }
        group_head(app, ui, place, i, g, head);
        group_body(app, ui, g.front, body);
        y = rect.bottom();
        if i + 1 < n {
            let split = Rect::from_min_max(pos2(full.left(), y), pos2(full.right(), y + SPLIT_H));
            divider(app, ui, place, i, split, room, total);
            y = split.bottom();
        }
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(layout_id(place), layout));
    ui.advance_cursor_after_rect(full);
}

/// A group's tabs across its top; at the right, the panel list (three
/// dots) and, on the first group, collapse.
fn group_head(
    app: &mut App,
    ui: &mut Ui,
    place: DockerPlace,
    gi: usize,
    g: &StackGroup,
    head: Rect,
) {
    let p = ui.painter().clone();
    p.rect_filled(head, 0.0, Tokens::tab_strip());
    p.hline(
        head.x_range(),
        head.bottom() - 0.5,
        egui::Stroke::new(1.0, Tokens::border()),
    );
    let buttons = if gi == 0 { 52.0 } else { 28.0 };
    let mut x = head.left() + 4.0;
    let font = egui::FontId::proportional(12.0);
    for &tab in &g.tabs {
        let name = tr(tab.key());
        let active = tab == g.front;
        let galley = p.layout_no_wrap(name.clone(), font.clone(), Tokens::text());
        let w = (galley.size().x + 24.0).min((head.right() - buttons - x).max(24.0));
        if x + 24.0 > head.right() - buttons {
            break;
        }
        let r = Rect::from_min_max(pos2(x, head.top() + 4.0), pos2(x + w, head.bottom()));
        let resp = ui.interact(
            r,
            egui::Id::new(("stack_tab", place as u8, tab.key())),
            Sense::click_and_drag(),
        );
        if active {
            p.rect_filled(
                r,
                egui::CornerRadius {
                    nw: 6,
                    ne: 6,
                    sw: 0,
                    se: 0,
                },
                Tokens::panel(),
            );
        } else if resp.hovered() {
            p.rect_filled(r.shrink2(vec2(0.0, 2.0)), 5.0, Tokens::tool_hover());
        }
        let color = if active {
            Tokens::text()
        } else {
            Tokens::text_dim()
        };
        p.with_clip_rect(r.shrink2(vec2(6.0, 0.0))).galley(
            pos2(r.left() + 12.0, r.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        // Close, while the pointer is over the tab.
        let close = Rect::from_center_size(pos2(r.right() - 9.0, r.center().y), vec2(14.0, 14.0));
        let over_close = resp.hover_pos().is_some_and(|q| close.contains(q));
        if resp.hovered() && w > 50.0 {
            let c = close.center();
            let col = if over_close {
                Tokens::text()
            } else {
                Tokens::text_dim()
            };
            let s = egui::Stroke::new(1.2, col);
            p.line_segment([c + vec2(-3.0, -3.0), c + vec2(3.0, 3.0)], s);
            p.line_segment([c + vec2(3.0, -3.0), c + vec2(-3.0, 3.0)], s);
        }
        let resp = if over_close {
            resp.rich_tip(tr("docker.close_docker"))
        } else {
            resp.rich_tip(format!("{name}\n{}", tr("docker.tab_desc")))
        };
        if resp.drag_started_by(egui::PointerButton::Primary) {
            let grab = resp
                .interact_pointer_pos()
                .map(|q| q - r.min)
                .unwrap_or(vec2(12.0, 12.0));
            app.docker_drag = Some(DockerDrag {
                tab,
                grab: vec2(grab.x.min(60.0), grab.y.min(HEAD_H)),
                last: None,
            });
        }
        if resp.clicked() {
            if over_close && w > 50.0 {
                app.close_docker(tab);
            } else {
                app.set_docker_column(place, true, Some(tab));
            }
        }
        egui::Popup::context_menu(&resp)
            .id(egui::Id::new(("stack_tab_menu", tab.key())))
            .style(crate::ui::menus::menu_popup_style)
            .show(|ui| {
                crate::ui::menus::body(ui, |ui| {
                    crate::ui::dockers::place_menu(app, ui, tab);
                    crate::ui::menus::sep(ui);
                    if crate::ui::menus::item(ui, "docker.close_docker", "", true) {
                        app.close_docker(tab);
                    }
                })
            });
        x += w + 2.0;
    }
    // The panel list.
    let more = Rect::from_center_size(
        pos2(head.right() - 15.0, head.center().y + 2.0),
        vec2(22.0, 22.0),
    );
    let resp = ui.interact(
        more,
        egui::Id::new(("stack_more", place as u8, gi)),
        Sense::click(),
    );
    if resp.hovered() {
        p.rect_filled(more, 5.0, Tokens::tool_hover());
    }
    for dx in [-4.5, 0.0, 4.5] {
        p.circle_filled(more.center() + vec2(dx, 0.0), 1.4, Tokens::text_dim());
    }
    let resp = resp.rich_tip(tr("docker.add_docker"));
    crate::ui::dockers::panels_menu(
        app,
        &resp,
        place,
        crate::ui::dockers::group_menu_id(place, gi),
    );
    if gi == 0 {
        let fold = Rect::from_center_size(
            pos2(head.right() - 39.0, head.center().y + 2.0),
            vec2(22.0, 22.0),
        );
        let resp = ui
            .interact(
                fold,
                egui::Id::new(("stack_fold", place as u8)),
                Sense::click(),
            )
            .rich_tip(tr("docker.collapse"));
        if resp.hovered() {
            p.rect_filled(fold, 5.0, Tokens::tool_hover());
        }
        let col = if resp.hovered() {
            Tokens::text()
        } else {
            Tokens::text_dim()
        };
        for pts in crate::ui::dockers::fold_arrows(
            fold.shrink(5.0),
            crate::ui::dockers::strip_at_left(place),
        ) {
            p.add(egui::epaint::PathShape::convex_polygon(
                pts.to_vec(),
                col,
                egui::Stroke::NONE,
            ));
        }
        if resp.clicked() {
            app.set_docker_column(place, false, None);
        }
    }
}

/// A group's page, kept to its part of the column.
fn group_body(app: &mut App, ui: &mut Ui, tab: DockerTab, body: Rect) {
    let inner = body.shrink2(vec2(8.0, 8.0));
    let mut page = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .id_salt(("stack_body", tab.key())),
    );
    page.set_clip_rect(body.intersect(ui.clip_rect()));
    if !app.drawing_shown() && tab != DockerTab::Hints {
        page.label(egui::RichText::new(tr("docker.no_drawing")).color(Tokens::text_dim()));
    } else {
        crate::ui::dockers::docker_page(app, &mut page, tab);
    }
}

/// The divider under group `gi`: dragging it gives height to one group
/// from the next.
fn divider(
    app: &mut App,
    ui: &mut Ui,
    place: DockerPlace,
    gi: usize,
    split: Rect,
    room: f32,
    total: f32,
) {
    let resp = ui
        .interact(
            split,
            egui::Id::new(("stack_split", place as u8, gi)),
            Sense::drag(),
        )
        .on_hover_cursor(egui::CursorIcon::ResizeVertical);
    let line = if resp.hovered() || resp.dragged() {
        Tokens::accent()
    } else {
        Tokens::border()
    };
    ui.painter().hline(
        split.x_range(),
        split.center().y,
        egui::Stroke::new(1.0, line),
    );
    if resp.dragged() && room > 0.0 {
        let dy = resp.drag_delta().y;
        let Some(i) = index(place) else { return };
        let groups = &mut app.stacks.columns[i];
        if gi + 1 >= groups.len() {
            return;
        }
        let dw = dy / room * total;
        let min = MIN_GROUP_H / room * total;
        let (a, b) = (groups[gi].weight, groups[gi + 1].weight);
        let na = (a + dw).clamp(min, a + b - min);
        groups[gi].weight = na;
        groups[gi + 1].weight = a + b - na;
    }
}

/// Width of a panel shown beside a folded column.
pub const FLYOUT_W: f32 = 300.0;

/// A collapsed stacked column: at its top a button that opens the whole
/// column, then the pictures of its panels, one under the other. A click
/// on a picture shows that panel beside the column, which stays folded
/// (a second click, Esc or a click elsewhere hides it); a picture
/// dragged moves its panel: along the column, to another column or out
/// to float.
pub fn strip(app: &mut App, ui: &mut Ui, place: DockerPlace) {
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, Tokens::tab_strip());
    let (tabs, _, _) = app.docker_column(place);
    let tabs = tabs.to_vec();
    // A flyout whose panel left the column goes.
    if app
        .stack_flyout
        .is_some_and(|(q, t)| q == place && !tabs.contains(&t))
    {
        app.stack_flyout = None;
    }
    let at_left = crate::ui::dockers::strip_at_left(place);
    // Open the whole column.
    let open = Rect::from_center_size(pos2(full.center().x, full.top() + 14.0), vec2(28.0, 20.0));
    let resp = ui
        .interact(
            open,
            egui::Id::new(("stack_strip_open", place as u8)),
            Sense::click(),
        )
        .rich_tip(tr("docker.expand"));
    if resp.hovered() {
        ui.painter().rect_filled(open, 5.0, Tokens::tool_hover());
    }
    let col = if resp.hovered() {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    for pts in crate::ui::dockers::fold_arrows(open.shrink2(vec2(8.0, 4.0)), !at_left) {
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            pts.to_vec(),
            col,
            egui::Stroke::NONE,
        ));
    }
    if resp.clicked() {
        app.stack_flyout = None;
        let front = app.docker_column(place).1;
        app.set_docker_column(place, true, Some(front));
    }
    ui.painter().hline(
        full.shrink2(vec2(6.0, 0.0)).x_range(),
        full.top() + 28.5,
        egui::Stroke::new(1.0, Tokens::border()),
    );
    let mut y = full.top() + 34.0;
    let mut laid = Vec::new();
    for tab in tabs {
        let r = Rect::from_center_size(pos2(full.center().x, y + 16.0), vec2(32.0, 32.0));
        laid.push((tab, r));
        let resp = ui
            .interact(
                r,
                egui::Id::new(("stack_strip", place as u8, tab.key())),
                Sense::click_and_drag(),
            )
            .rich_tip(format!("{}\n{}", tr(tab.key()), tr("docker.tab_desc")));
        let shown = app.stack_flyout == Some((place, tab));
        if shown {
            ui.painter().rect_filled(r, 6.0, Tokens::tool_active());
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 6.0, Tokens::tool_hover());
        }
        crate::ui::panel_icons::draw(
            ui.painter(),
            Rect::from_center_size(r.center(), vec2(18.0, 18.0)),
            tab,
        );
        if resp.drag_started_by(egui::PointerButton::Primary) {
            app.stack_flyout = None;
            app.docker_drag = Some(DockerDrag {
                tab,
                grab: vec2(12.0, 12.0),
                last: None,
            });
        }
        if resp.clicked() {
            app.stack_flyout = if shown { None } else { Some((place, tab)) };
        }
        y += 38.0;
        if y > full.bottom() - 40.0 {
            break;
        }
    }
    // The pictures as a strip of tabs: a dragged panel can be put among
    // them, in any order.
    let layout = crate::ui::dockers::StripLayout {
        frame: ui.ctx().cumulative_frame_nr(),
        strip: full,
        tabs: laid,
    };
    ui.ctx()
        .data_mut(|d| d.insert_temp(crate::ui::dockers::strip_id(place), layout));
    let more = Rect::from_center_size(pos2(full.center().x, y + 12.0), vec2(28.0, 22.0));
    let resp = ui.interact(
        more,
        crate::ui::dockers::more_button_id(place),
        Sense::click(),
    );
    if resp.hovered() {
        ui.painter().rect_filled(more, 5.0, Tokens::tool_hover());
    }
    for dx in [-4.5, 0.0, 4.5] {
        ui.painter()
            .circle_filled(more.center() + vec2(dx, 0.0), 1.4, Tokens::text_dim());
    }
    let resp = resp.rich_tip(tr("docker.add_docker"));
    crate::ui::dockers::panels_menu(
        app,
        &resp,
        place,
        crate::ui::dockers::more_menu_id(place),
    );
    if let Some((q, tab)) = app.stack_flyout {
        if q == place {
            flyout(app, ui.ctx(), place, tab, full, at_left);
        }
    }
    ui.advance_cursor_after_rect(full);
}

/// The panel `tab` beside the folded column at `place` (whose strip is
/// `strip`), over the drawing: a title with buttons to dock it open and
/// to hide it, then its page.
fn flyout(
    app: &mut App,
    ctx: &egui::Context,
    place: DockerPlace,
    tab: DockerTab,
    strip: Rect,
    at_left: bool,
) {
    let h = strip.height().clamp(160.0, 640.0);
    let x = if at_left {
        strip.right()
    } else {
        strip.left() - FLYOUT_W
    };
    let rect = Rect::from_min_size(pos2(x, strip.top()), vec2(FLYOUT_W, h));
    let id = egui::Id::new(("stack_flyout", place as u8));
    let mut hide = false;
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            let p = ui.painter();
            p.rect(
                rect,
                8.0,
                Tokens::panel(),
                egui::Stroke::new(1.0, Tokens::window_border()),
                egui::StrokeKind::Inside,
            );
            let head = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + HEAD_H));
            p.text(
                pos2(head.left() + 12.0, head.center().y),
                egui::Align2::LEFT_CENTER,
                tr(tab.key()),
                crate::theme::bold(12.0),
                Tokens::text(),
            );
            p.hline(
                head.shrink2(vec2(1.0, 0.0)).x_range(),
                head.bottom() - 0.5,
                egui::Stroke::new(1.0, Tokens::border()),
            );
            // Hide.
            let close = Rect::from_center_size(
                pos2(head.right() - 15.0, head.center().y),
                vec2(22.0, 22.0),
            );
            let resp = ui
                .interact(close, id.with("close"), Sense::click())
                .rich_tip(tr("docker.flyout_hide"));
            if resp.hovered() {
                ui.painter().rect_filled(close, 5.0, Tokens::tool_hover());
            }
            let c = close.center();
            let s = egui::Stroke::new(1.3, Tokens::text_dim());
            ui.painter()
                .line_segment([c + vec2(-3.5, -3.5), c + vec2(3.5, 3.5)], s);
            ui.painter()
                .line_segment([c + vec2(3.5, -3.5), c + vec2(-3.5, 3.5)], s);
            if resp.clicked() {
                hide = true;
            }
            // Dock the column open with this panel in front.
            let dock = Rect::from_center_size(
                pos2(head.right() - 39.0, head.center().y),
                vec2(22.0, 22.0),
            );
            let resp = ui
                .interact(dock, id.with("dock"), Sense::click())
                .rich_tip(tr("docker.expand"));
            if resp.hovered() {
                ui.painter().rect_filled(dock, 5.0, Tokens::tool_hover());
            }
            for pts in crate::ui::dockers::fold_arrows(dock.shrink(5.0), !at_left) {
                ui.painter().add(egui::epaint::PathShape::convex_polygon(
                    pts.to_vec(),
                    Tokens::text_dim(),
                    egui::Stroke::NONE,
                ));
            }
            if resp.clicked() {
                hide = true;
                app.set_docker_column(place, true, Some(tab));
            }
            // Arcano edition: pin it open (a click on the drawing then
            // leaves it), or let it fold away again.
            let pin = Rect::from_center_size(
                pos2(head.right() - 63.0, head.center().y),
                vec2(22.0, 22.0),
            );
            let pinned = app.settings.flyout_pinned;
            let resp = ui
                .interact(pin, id.with("pin"), Sense::click())
                .rich_tip(crate::ui::tips::pair(
                    if pinned { "docker.unpin" } else { "docker.pin" },
                    "docker.pin_desc",
                ));
            if pinned {
                ui.painter().rect_filled(pin, 5.0, Tokens::selected_row());
            } else if resp.hovered() {
                ui.painter().rect_filled(pin, 5.0, Tokens::tool_hover());
            }
            paint_pin(ui.painter(), pin.shrink(5.0), if pinned { Tokens::accent() } else { Tokens::text_dim() });
            if resp.clicked() {
                app.settings.flyout_pinned = !pinned;
            }
            let body = Rect::from_min_max(pos2(rect.left(), head.bottom()), rect.max);
            let inner = body.shrink2(vec2(8.0, 8.0));
            let mut page = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .id_salt(("stack_flyout_body", tab.key())),
            );
            page.set_clip_rect(body.shrink(1.0));
            if !app.drawing_shown() && tab != DockerTab::Hints {
                page.label(egui::RichText::new(tr("docker.no_drawing")).color(Tokens::text_dim()));
            } else {
                egui::ScrollArea::vertical()
                    .id_salt(("stack_flyout_scroll", tab.key()))
                    .show(&mut page, |ui| {
                        crate::ui::dockers::docker_page(app, ui, tab);
                    });
            }
        });
    // A click elsewhere or Esc hides it; a popup it opened (a menu, a
    // colour picker) keeps it.
    let (clicked, esc, at) = ctx.input(|i| {
        (
            i.pointer.any_pressed(),
            i.key_pressed(egui::Key::Escape),
            i.pointer.interact_pos(),
        )
    });
    let outside = at.is_some_and(|q| !rect.contains(q) && !strip.contains(q));
    let popup = egui::Popup::is_any_open(ctx);
    // A click in a window over the drawing (a dialog the panel opened)
    // is not a click on the drawing.
    let on_window = at.is_some_and(|q| {
        ctx.layer_id_at(q)
            .is_some_and(|l| matches!(l.order, egui::Order::Foreground | egui::Order::Tooltip) && l.id != id)
    });
    let pinned = app.settings.flyout_pinned;
    if hide || (esc && !pinned) || (clicked && outside && !popup && !on_window && !pinned) {
        app.stack_flyout = None;
    }
}

/// A pin.
fn paint_pin(p: &egui::Painter, r: Rect, c: egui::Color32) {
    let s = egui::Stroke::new(1.3, c);
    let head = Rect::from_center_size(pos2(r.center().x, r.top() + 3.5), vec2(7.0, 6.0));
    p.rect_stroke(head, 1.0, s, egui::StrokeKind::Inside);
    p.line_segment([pos2(r.left() + 1.0, head.bottom()), pos2(r.right() - 1.0, head.bottom())], s);
    p.line_segment([pos2(r.center().x, head.bottom()), pos2(r.center().x, r.bottom())], s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_follow_the_column_and_regroup() {
        let mut s = Stacks::default();
        let p = DockerPlace::Right;
        s.follow(
            p,
            &[DockerTab::Hints, DockerTab::Properties, DockerTab::Objects],
            DockerTab::Hints,
        );
        assert_eq!(s.groups(p).len(), 1);
        // Objects starts a group under the first.
        s.regroup(p, DockerTab::Objects, 0, true);
        assert_eq!(s.groups(p).len(), 2);
        assert_eq!(s.groups(p)[1].tabs, vec![DockerTab::Objects]);
        // Properties joins Objects' group, in front.
        s.regroup(p, DockerTab::Properties, 1, false);
        assert_eq!(s.groups(p)[0].tabs, vec![DockerTab::Hints]);
        assert_eq!(
            s.groups(p)[1].tabs,
            vec![DockerTab::Objects, DockerTab::Properties]
        );
        assert_eq!(s.groups(p)[1].front, DockerTab::Properties);
        // A panel closed leaves its group; one opened joins the front's.
        s.follow(
            p,
            &[DockerTab::Hints, DockerTab::Objects, DockerTab::Color],
            DockerTab::Objects,
        );
        assert_eq!(
            s.groups(p)[1].tabs,
            vec![DockerTab::Objects, DockerTab::Color]
        );
        assert_eq!(s.groups(p)[1].front, DockerTab::Objects);
        // Hints moved into the second group: the first one goes.
        s.regroup(p, DockerTab::Hints, 1, false);
        assert_eq!(s.groups(p).len(), 1);
    }

    #[test]
    fn a_stacked_column_draws_its_groups_in_its_width() {
        let mut app = App::headless();
        app.settings.stacked_panels = true;
        app.open_dockers = vec![DockerTab::Hints, DockerTab::Properties, DockerTab::Objects];
        app.show_dockers = true;
        app.stacks.follow(
            DockerPlace::Right,
            &app.open_dockers.clone(),
            DockerTab::Hints,
        );
        app.stacks
            .regroup(DockerPlace::Right, DockerTab::Objects, 0, true);
        let ctx = crate::theme::ui_context();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1400.0, 800.0));
        let mut widths = Vec::new();
        for _ in 0..5 {
            let mut o = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    let r = egui::Panel::right("dock")
                        .default_size(300.0)
                        .show(ui, |ui| column(&mut app, ui, DockerPlace::Right));
                    widths.push(r.response.rect.width());
                },
            );
            o.textures_delta.clear();
        }
        assert!(widths.iter().all(|w| *w < 420.0), "{widths:?}");
        assert!((widths[4] - widths[2]).abs() < 1.0, "{widths:?}");
        let l = ctx
            .data(|d| d.get_temp::<StackLayout>(layout_id(DockerPlace::Right)))
            .unwrap();
        let (h0, h1) = (l.heads[0].unwrap(), l.heads[1].unwrap());
        assert!(
            h1.top() > h0.bottom() + MIN_GROUP_H - 1.0,
            "two groups, one under the other"
        );
        // A drop on the second group's lower half starts a third group.
        let p = pos2(h1.center().x, screen.bottom() - 20.0);
        let (place, g, new_after, _) = drop_on(&ctx, p).unwrap();
        assert_eq!((place, g, new_after), (DockerPlace::Right, 1, true));
    }
}
