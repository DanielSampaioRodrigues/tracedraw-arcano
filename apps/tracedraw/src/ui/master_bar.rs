//! Arcano edition: the bar over the drawing while a master page is edited
//! (see `App::edit_master`): which master, and the way back to the pages.
//! With facing pages both pages of the master's spread are at hand: a
//! press on either works there (`App::master_press`).
//! See docs/behavior/pages-panel.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::masters::label;
use crate::theme::Tokens;
use crate::ui::tips::{pair, RichTip as _};
use egui::{pos2, Rect, Ui};

/// The bar and the click on the other side, while a master is edited.
pub fn master_bar(app: &mut App, ui: &mut Ui, canvas: Rect) {
    let Some(v) = app.master_view else { return };
    let Some(m) = app.doc().master_page(v.master) else {
        app.leave_master();
        return;
    };
    let mut back = false;
    egui::Area::new(egui::Id::new("arcano_master_bar"))
        .order(egui::Order::Middle)
        .pivot(egui::Align2::CENTER_TOP)
        .fixed_pos(pos2(canvas.center().x, canvas.top() + 8.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(Tokens::panel())
                .stroke(egui::Stroke::new(1.5, Tokens::accent()))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(trf("master.editing", &[("m", &label(&m))]))
                                .font(crate::theme::bold(12.0))
                                .color(Tokens::accent()),
                        );
                        ui.separator();
                        if ui
                            .button(tr("master.back"))
                            .rich_tip(pair("master.back", "master.back_desc"))
                            .clicked()
                        {
                            back = true;
                        }
                    });
                });
        });
    if back {
        app.leave_master();
    }
}
