//! Drop-down lists with the interface's arrow: a thin chevron instead of
//! a filled triangle. Every list of the program is made here.

use egui::style::WidgetVisuals;
use egui::{Painter, Rect, Stroke};

/// A list identified by `id`.
pub fn from_id_salt(id: impl egui::AsIdSalt) -> egui::ComboBox {
    egui::ComboBox::from_id_salt(id).icon(chevron)
}

/// A list with a label beside it.
pub fn from_label(label: impl Into<egui::WidgetText>) -> egui::ComboBox {
    egui::ComboBox::from_label(label).icon(chevron)
}

/// The arrow: a thin chevron pointing down (up while the list is open
/// above the field), in the field's text colour.
pub fn chevron(ui: &egui::Ui, rect: Rect, visuals: &WidgetVisuals, is_open: bool) {
    paint(ui.painter(), rect, visuals.fg_stroke.color, is_open);
}

fn paint(painter: &Painter, rect: Rect, color: egui::Color32, up: bool) {
    let c = rect.center();
    let w = (rect.width().min(rect.height()) * 0.32).clamp(3.0, 5.0);
    let h = w * 0.55;
    let s = if up { -1.0 } else { 1.0 };
    painter.add(egui::Shape::line(
        vec![
            c + egui::vec2(-w, -h * s),
            c + egui::vec2(0.0, h * s),
            c + egui::vec2(w, -h * s),
        ],
        Stroke::new(1.4, color),
    ));
}
