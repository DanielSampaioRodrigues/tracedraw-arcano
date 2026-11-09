//! Underline and Strikethrough Options: the look of one of the two lines
//! of the characters in a window of its own, with On, Weight, Type, Offset
//! and Colour, as the panel's dialogs have them. What is set shows on the
//! text as it is set; Cancel puts back what was there when it opened.
//! Opened from the text bar's arrow beside the Underline and Strikethrough
//! buttons and from the Text docker's rows. See docs/behavior/text.md.

use crate::app::App;
use crate::i18n::tr;
use crate::ui::dialogs::window;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use crate::ui::text_docker::{line_kind_name, Line};
use egui::Context;
use tracedraw_core::document::{LineKind, LineStyle};
use tracedraw_core::{Command, ShapeId, ShapeKind};

/// The dialog's state: which line, how it looks, whether it is on, and the
/// texts as they were when it opened (Cancel puts them back).
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub which: Line,
    pub on: bool,
    pub style: LineStyle,
    pub before: Vec<(ShapeId, ShapeKind)>,
}

impl State {
    /// Open on the look of the selected text (its first character),
    /// remembering the texts to put back.
    pub fn open(app: &mut App, which: Line) -> State {
        let mut before = Vec::new();
        let mut style = LineStyle::default();
        let mut on = false;
        let mut first = true;
        for (id, _) in app.text_style_targets() {
            let Some(shape) = app.doc().find_shape(id).cloned() else {
                continue;
            };
            if let ShapeKind::Text { spans, .. } = &shape.kind {
                if let (true, Some(sp)) = (first, spans.first()) {
                    first = false;
                    on = match which {
                        Line::Underline => sp.underline,
                        Line::Strike => sp.strikethrough,
                    };
                    style = match which {
                        Line::Underline => sp.underline_style.clone(),
                        Line::Strike => sp.strike_style.clone(),
                    };
                }
            }
            before.push((id, shape.kind.clone()));
        }
        State {
            which,
            on,
            style,
            before,
        }
    }

    /// Give the selected text the look being set (one undo step, merged
    /// with the others of the same text).
    pub fn apply(&self, app: &mut App) {
        let (on, style, which) = (self.on, self.style.clone(), self.which);
        crate::ui::text_docker::edit_spans(app, move |sp| match which {
            Line::Underline => {
                sp.underline = on;
                sp.underline_style = style.clone();
            }
            Line::Strike => {
                sp.strikethrough = on;
                sp.strike_style = style.clone();
            }
        });
    }

    /// Put the texts back as they were when the dialog opened.
    pub fn restore(&self, app: &mut App) {
        let cmds: Vec<Command> = self
            .before
            .iter()
            .map(|(id, kind)| Command::SetShapeKind {
                shape: *id,
                kind: kind.clone(),
            })
            .collect();
        if !cmds.is_empty() {
            let _ = app.engine.run_batch("Line Options", &cmds);
        }
    }

    /// The title of the window.
    pub fn title(&self) -> String {
        tr(match self.which {
            Line::Underline => "line_opt.underline",
            Line::Strike => "line_opt.strike",
        })
    }
}

/// Draw the window; `close` closes it.
pub fn dialog(app: &mut App, ctx: &Context, st: &mut State, close: &mut bool) {
    let black = app.doc().metadata.primary_color_mode.black();
    let mut changed = false;
    let mut cancel = false;
    window(ctx, st.title()).show(ctx, |ui| {
        ui.set_max_width(330.0);
        egui::Grid::new("line_options")
            .num_columns(2)
            .spacing([10.0, 7.0])
            .show(ui, |ui| {
                if ui
                    .checkbox(&mut st.on, tr("line_opt.on"))
                    .rich_tip(tr("line_opt.on_tip"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
                ui.label(tr("line_opt.weight"));
                let mut thick = st.style.thickness_pt.unwrap_or(0.0);
                if ui
                    .add_sized(
                        [90.0, 22.0],
                        NumField::new(&mut thick)
                            .range(0.0..=30.0)
                            .max_decimals(1)
                            .suffix(" pt".to_string()),
                    )
                    .rich_tip(tr("docker.line_thickness"))
                    .changed()
                {
                    st.style.thickness_pt = (thick > 0.0).then_some(thick);
                    changed = true;
                }
                ui.end_row();
                ui.label(tr("line_opt.type"));
                egui::ComboBox::from_id_salt("line_options_kind")
                    .width(130.0)
                    .selected_text(line_kind_name(st.style.kind))
                    .show_ui(ui, |ui| {
                        for kind in LineKind::ALL {
                            if ui
                                .selectable_label(
                                    st.style.kind == kind,
                                    line_kind_name(kind),
                                )
                                .clicked()
                            {
                                st.style.kind = kind;
                                changed = true;
                            }
                        }
                    })
                    .response
                    .rich_tip(tr("docker.line_kind"));
                ui.end_row();
                ui.label(tr("line_opt.offset"));
                let mut place = st.style.offset_pt.unwrap_or(0.0);
                if ui
                    .add_sized(
                        [90.0, 22.0],
                        NumField::new(&mut place)
                            .range(-30.0..=30.0)
                            .max_decimals(1)
                            .suffix(" pt".to_string()),
                    )
                    .rich_tip(tr("docker.line_position"))
                    .changed()
                {
                    st.style.offset_pt = (place != 0.0).then_some(place);
                    changed = true;
                }
                ui.end_row();
                ui.label(tr("line_opt.colour"));
                ui.horizontal(|ui| {
                    let mut own = st.style.color.is_some();
                    if ui
                        .checkbox(&mut own, tr("line_opt.own"))
                        .rich_tip(tr("docker.line_own_colour"))
                        .changed()
                    {
                        st.style.color = own.then_some(black);
                        changed = true;
                    }
                    if let Some(c) = &mut st.style.color {
                        if crate::ui::color_picker::color_button(ui, c) {
                            changed = true;
                        }
                    }
                });
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .button(tr("dialog.ok"))
                .rich_tip(tr("dialog.ok"))
                .clicked()
            {
                *close = true;
            }
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(tr("line_opt.cancel_tip"))
                .clicked()
            {
                cancel = true;
                *close = true;
            }
        });
    });
    if cancel {
        st.restore(app);
    } else if changed {
        st.apply(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::Color;
    use tracedraw_core::{ParagraphStyle, TextAlign, TextSpan};

    fn with_text(app: &mut App, s: &str) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new(s, "Sans", 12.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 80.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        id
    }

    fn span_of(app: &App, id: ShapeId) -> TextSpan {
        match &app.doc().find_shape(id).expect("text").kind {
            ShapeKind::Text { spans, .. } => spans.first().cloned().expect("a span"),
            k => panic!("{k:?}"),
        }
    }

    /// The dialog opens on the selected text's look, gives it the one it
    /// sets and puts back what was there when it is cancelled.
    #[test]
    fn the_options_set_the_look_and_cancel_puts_it_back() {
        let mut app = App::headless();
        let id = with_text(&mut app, "Um texto");
        let mut st = State::open(&mut app, Line::Underline);
        assert!(!st.on && st.style.is_plain());
        st.on = true;
        st.style.kind = LineKind::Wavy;
        st.style.thickness_pt = Some(1.2);
        st.style.offset_pt = Some(-3.0);
        st.style.color = Some(Color::rgb8(200, 0, 0));
        st.apply(&mut app);
        let sp = span_of(&app, id);
        assert!(sp.underline);
        assert_eq!(sp.underline_style, st.style);
        // Cancel puts the text back as it was.
        st.restore(&mut app);
        let sp = span_of(&app, id);
        assert!(!sp.underline && sp.underline_style.is_plain());
        // The strike line has its own state.
        let mut st = State::open(&mut app, Line::Strike);
        assert!(!st.on);
        st.on = true;
        st.style.kind = LineKind::Double;
        st.apply(&mut app);
        let sp = span_of(&app, id);
        assert!(sp.strikethrough && sp.strike_style.kind == LineKind::Double);
        assert!(sp.underline_style.is_plain(), "the underline stays as it was");
    }

    /// With no text selected the dialog opens on the plain look and does
    /// nothing.
    #[test]
    fn with_no_text_the_options_do_nothing() {
        let mut app = App::headless();
        let mut st = State::open(&mut app, Line::Underline);
        assert!(st.before.is_empty() && !st.on && st.style.is_plain());
        st.on = true;
        st.apply(&mut app);
        st.restore(&mut app);
    }
}
