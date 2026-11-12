//! Underline and Strikethrough Options: the look of one of the two lines
//! of the characters in a window of its own, with On, Weight, Type, Offset,
//! Colour at a tint, the gaps of a dashed or dotted line and the two
//! overprints. What is set shows on the text as it is set, and only the
//! field that is touched is set: a selection whose lines differ keeps what
//! it has in the fields left alone. Cancel puts back what was there when
//! the window opened. Opened from the text bar's arrow beside the
//! Underline and Strikethrough buttons and from the Text docker's rows.
//! See docs/behavior/text.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::window;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use crate::ui::text_docker::{line_kind_name, Line};
use egui::{Context, Ui};
use tracedraw_core::document::{LineKind, LineLook, LineStyle};
use tracedraw_core::{Command, ShapeId, ShapeKind, TextSpan};

/// The fields of the window the selection has in more than one value: they
/// show as mixed and are only set when touched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mixed {
    pub on: bool,
    pub weight: bool,
    pub kind: bool,
    pub offset: bool,
    pub colour: bool,
    pub tint: bool,
    pub gap: bool,
    pub gap_tint: bool,
    pub overprint: bool,
    pub gap_overprint: bool,
}

/// The window's state: which line, how it looks, whether it is on, which
/// fields are mixed, and the texts as they were when it opened (Cancel puts
/// them back).
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub which: Line,
    pub on: bool,
    pub style: LineLook,
    pub before: Vec<(ShapeId, ShapeKind)>,
    pub mixed: Mixed,
}

/// The line of a span the window is about.
fn look_of(s: &TextSpan, which: Line) -> (bool, LineLook) {
    match which {
        Line::Underline => (s.underline.is_on(), s.underline_style.clone()),
        Line::Strike => (s.strikethrough.is_on(), s.strike_style.clone()),
    }
}

impl State {
    /// Open on the look of the selected text (its first character), remember
    /// the texts to put back, and note the mixed fields.
    pub fn open(app: &mut App, which: Line) -> State {
        let mut before = Vec::new();
        let mut looks: Vec<(bool, LineLook)> = Vec::new();
        for (id, _) in app.text_style_targets() {
            let Some(shape) = app.doc().find_shape(id).cloned() else {
                continue;
            };
            if let ShapeKind::Text { spans, .. } = &shape.kind {
                for s in spans {
                    if s.char_count() > 0 {
                        looks.push(look_of(s, which));
                    }
                }
            }
            before.push((id, shape.kind.clone()));
        }
        let (on, style) = looks.first().cloned().unwrap_or_default();
        let mut mixed = Mixed::default();
        for (other_on, other) in looks.iter().skip(1) {
            mixed.on |= *other_on != on;
            mixed.kind |= other.kind != style.kind;
            mixed.weight |= other.thickness_pt != style.thickness_pt;
            mixed.offset |= other.offset_pt != style.offset_pt;
            mixed.colour |= other.color != style.color;
            mixed.tint |= other.tint != style.tint;
            mixed.gap |= other.gap_color != style.gap_color;
            mixed.gap_tint |= other.gap_tint != style.gap_tint;
            mixed.overprint |= other.overprint != style.overprint;
            mixed.gap_overprint |= other.gap_overprint != style.gap_overprint;
        }
        State {
            which,
            on,
            style,
            before,
            mixed,
        }
    }

    /// Put the texts back as they were when the window opened.
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

    /// Give the selected text the whole look being set (the tests set the
    /// whole look at once; the window sets one field at a time).
    #[cfg(test)]
    pub fn apply(&self, app: &mut App) {
        let (on, style, which) = (self.on, self.style.clone(), self.which);
        crate::ui::text_docker::edit_spans(app, move |sp| match which {
            Line::Underline => {
                sp.underline = turned(sp.underline, on);
                sp.underline_style = style.clone();
            }
            Line::Strike => {
                sp.strikethrough = turned(sp.strikethrough, on);
                sp.strike_style = style.clone();
            }
        });
    }
}

/// Turn the line on or off for the chosen characters.
fn set_on(app: &mut App, which: Line, on: bool) {
    crate::ui::text_docker::edit_spans(app, move |sp| match which {
        Line::Underline => sp.underline = turned(sp.underline, on),
        Line::Strike => sp.strikethrough = turned(sp.strikethrough, on),
    });
}

/// A line style turned on or off: one already on keeps its style (a
/// double line stays double), one turned on is a single thin line.
fn turned(style: LineStyle, on: bool) -> LineStyle {
    if style.is_on() == on {
        style
    } else {
        LineStyle::from(on)
    }
}

/// Set one field of the line's look, leaving the others as each character
/// has them (a mixed selection keeps what it was not told to change).
fn set_line(app: &mut App, which: Line, f: impl Fn(&mut LineLook) + Clone + 'static) {
    crate::ui::text_docker::edit_spans(app, move |sp| {
        let st = match which {
            Line::Underline => &mut sp.underline_style,
            Line::Strike => &mut sp.strike_style,
        };
        f(st);
    });
}

/// The mark after a field the selection has in more than one value.
fn mixed_mark(ui: &mut Ui, mixed: bool) {
    if mixed {
        ui.label(
            egui::RichText::new(tr("line_opt.mixed"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
    }
}

/// Draw the window; `close` closes it.
pub fn dialog(app: &mut App, ctx: &Context, st: &mut State, close: &mut bool) {
    let black = app.doc().metadata.primary_color_mode.black();
    let which = st.which;
    let mut cancel = false;
    window(ctx, st.title()).show(ctx, |ui| {
        ui.set_max_width(360.0);
        egui::Grid::new("line_options")
            .num_columns(2)
            .spacing([10.0, 7.0])
            .show(ui, |ui| {
                // On.
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut st.on, tr("line_opt.on"))
                        .rich_tip(tr("line_opt.on_tip"))
                        .changed()
                    {
                        set_on(app, which, st.on);
                    }
                    mixed_mark(ui, st.mixed.on);
                });
                ui.end_row();
                // Weight.
                ui.label(tr("line_opt.weight"));
                ui.horizontal(|ui| {
                    let mut thick = st.style.thickness_pt.unwrap_or(0.0);
                    if ui
                        .add_sized(
                            [90.0, 22.0],
                            NumField::new(&mut thick)
                                .range(0.0..=30.0)
                                .max_decimals(2)
                                .suffix(" pt".to_string()),
                        )
                        .rich_tip(tr("docker.line_thickness"))
                        .changed()
                    {
                        let v = (thick > 0.0).then_some(thick);
                        st.style.thickness_pt = v;
                        set_line(app, which, move |s| s.thickness_pt = v);
                    }
                    mixed_mark(ui, st.mixed.weight);
                });
                ui.end_row();
                // Type.
                ui.label(tr("line_opt.type"));
                ui.horizontal(|ui| {
                    let mut picked = None;
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
                                    picked = Some(kind);
                                }
                            }
                        })
                        .response
                        .rich_tip(tr("docker.line_kind"));
                    if let Some(kind) = picked {
                        st.style.kind = kind;
                        set_line(app, which, move |s| s.kind = kind);
                    }
                    mixed_mark(ui, st.mixed.kind);
                });
                ui.end_row();
                // Where it sits.
                ui.label(tr("line_opt.offset"));
                ui.horizontal(|ui| {
                    let mut place = st.style.offset_pt.unwrap_or(0.0);
                    if ui
                        .add_sized(
                            [90.0, 22.0],
                            NumField::new(&mut place)
                                .range(-30.0..=30.0)
                                .max_decimals(2)
                                .suffix(" pt".to_string()),
                        )
                        .rich_tip(tr("docker.line_position"))
                        .changed()
                    {
                        let v = (place != 0.0).then_some(place);
                        st.style.offset_pt = v;
                        set_line(app, which, move |s| s.offset_pt = v);
                    }
                    mixed_mark(ui, st.mixed.offset);
                });
                ui.end_row();
                // Colour at its tint.
                ui.label(tr("line_opt.colour"));
                ui.horizontal(|ui| {
                    let mut own = st.style.color.is_some();
                    if ui
                        .checkbox(&mut own, tr("line_opt.own"))
                        .rich_tip(tr("docker.line_own_colour"))
                        .changed()
                    {
                        let c = own.then_some(black);
                        st.style.color = c;
                        set_line(app, which, move |s| s.color = c);
                    }
                    if let Some(c) = &mut st.style.color {
                        let mut c = *c;
                        if crate::ui::color_picker::color_button(ui, &mut c) {
                            st.style.color = Some(c);
                            set_line(app, which, move |s| s.color = Some(c));
                        }
                        ui.label(tr("line_opt.tint"));
                        let mut tint = st.style.tint;
                        if ui
                            .add_sized(
                                [52.0, 22.0],
                                NumField::new(&mut tint)
                                    .range(0.0..=100.0)
                                    .max_decimals(0)
                                    .suffix(" %".to_string()),
                            )
                            .rich_tip(tr("line_opt.tint_tip"))
                            .changed()
                        {
                            st.style.tint = tint;
                            set_line(app, which, move |s| s.tint = tint);
                        }
                    }
                    mixed_mark(ui, st.mixed.colour || st.mixed.tint);
                });
                ui.end_row();
                // Overprint of the line.
                ui.label("");
                ui.horizontal(|ui| {
                    let mut op = st.style.overprint;
                    if ui
                        .checkbox(&mut op, tr("line_opt.overprint"))
                        .rich_tip(tr("line_opt.overprint_tip"))
                        .changed()
                    {
                        st.style.overprint = op;
                        set_line(app, which, move |s| s.overprint = op);
                    }
                    mixed_mark(ui, st.mixed.overprint);
                });
                ui.end_row();
                // The gaps of a dashed or dotted line.
                ui.label(tr("line_opt.gap"));
                ui.horizontal(|ui| {
                    let mut own = st.style.gap_color.is_some();
                    if ui
                        .checkbox(&mut own, tr("line_opt.own"))
                        .rich_tip(tr("line_opt.gap_tip"))
                        .changed()
                    {
                        let c = own.then_some(black);
                        st.style.gap_color = c;
                        set_line(app, which, move |s| s.gap_color = c);
                    }
                    if let Some(c) = &mut st.style.gap_color {
                        let mut c = *c;
                        if crate::ui::color_picker::color_button(ui, &mut c) {
                            st.style.gap_color = Some(c);
                            set_line(app, which, move |s| s.gap_color = Some(c));
                        }
                        ui.label(tr("line_opt.tint"));
                        let mut tint = st.style.gap_tint;
                        if ui
                            .add_sized(
                                [52.0, 22.0],
                                NumField::new(&mut tint)
                                    .range(0.0..=100.0)
                                    .max_decimals(0)
                                    .suffix(" %".to_string()),
                            )
                            .rich_tip(tr("line_opt.gap_tint_tip"))
                            .changed()
                        {
                            st.style.gap_tint = tint;
                            set_line(app, which, move |s| s.gap_tint = tint);
                        }
                    }
                    mixed_mark(ui, st.mixed.gap || st.mixed.gap_tint);
                });
                ui.end_row();
                // Overprint of the gaps.
                ui.label("");
                ui.horizontal(|ui| {
                    let mut op = st.style.gap_overprint;
                    if ui
                        .checkbox(&mut op, tr("line_opt.gap_overprint"))
                        .rich_tip(tr("line_opt.gap_overprint_tip"))
                        .changed()
                    {
                        st.style.gap_overprint = op;
                        set_line(app, which, move |s| s.gap_overprint = op);
                    }
                    mixed_mark(ui, st.mixed.gap_overprint);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::{Color, ParagraphStyle, TextAlign};

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

    /// The window opens on the selected text's look, gives it what it sets
    /// field by field and puts back what was there when it is cancelled.
    #[test]
    fn the_options_set_the_look_and_cancel_puts_it_back() {
        let mut app = App::headless();
        let id = with_text(&mut app, "Um texto");
        let mut st = State::open(&mut app, Line::Underline);
        assert!(!st.on && st.style.is_plain() && st.mixed == Mixed::default());
        st.on = true;
        set_on(&mut app, Line::Underline, true);
        st.style.kind = LineKind::Wavy;
        set_line(&mut app, Line::Underline, |s| s.kind = LineKind::Wavy);
        set_line(&mut app, Line::Underline, |s| s.thickness_pt = Some(1.2));
        set_line(&mut app, Line::Underline, |s| {
            s.color = Some(Color::rgb8(200, 0, 0));
            s.tint = 40.0;
            s.overprint = true;
        });
        let sp = span_of(&app, id);
        assert!(sp.underline.is_on());
        assert_eq!(sp.underline_style.kind, LineKind::Wavy);
        assert_eq!(sp.underline_style.thickness_pt, Some(1.2));
        assert_eq!(sp.underline_style.tint, 40.0);
        assert!(sp.underline_style.overprint);
        assert_eq!(sp.underline_style.color, Some(Color::rgb8(200, 0, 0)));
        // Cancel puts the text back as it was.
        st.restore(&mut app);
        let sp = span_of(&app, id);
        assert!(!sp.underline.is_on() && sp.underline_style.is_plain());
        // The gaps have their colour and tint; the strike line is its own.
        st.style.kind = LineKind::Dashed;
        st.style.gap_color = Some(Color::rgb8(0, 0, 255));
        st.style.gap_tint = 30.0;
        st.apply(&mut app);
        let sp = span_of(&app, id);
        assert_eq!(sp.underline_style.gap_tint, 30.0);
        let mut st = State::open(&mut app, Line::Strike);
        assert!(!st.on);
        st.on = true;
        st.style.kind = LineKind::Double;
        st.apply(&mut app);
        let sp = span_of(&app, id);
        assert!(sp.strikethrough.is_on() && sp.strike_style.kind == LineKind::Double);
    }

    /// A selection whose lines differ shows those fields as mixed and only
    /// the field touched is set.
    #[test]
    fn a_mixed_selection_keeps_the_fields_left_alone() {
        let mut app = App::headless();
        let mut a = TextSpan::new("um", "Sans", 12.0);
        a.underline = LineStyle::SingleThin;
        a.underline_style = LineLook {
            kind: LineKind::Wavy,
            thickness_pt: Some(1.0),
            color: Some(Color::rgb8(200, 0, 0)),
            ..Default::default()
        };
        let mut b = TextSpan::new("dois", "Sans", 12.0);
        b.underline = LineStyle::None;
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![a, b],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 80.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        let st = State::open(&mut app, Line::Underline);
        assert!(st.mixed.on && st.mixed.weight && st.mixed.colour);
        // One is wavy and the other a plain single line, so the kind is
        // mixed too; the weight and the colour are.
        assert!(st.mixed.kind && st.mixed.offset == false && st.mixed.tint == false);
        // Changing only the weight leaves on and off as they were.
        set_line(&mut app, Line::Underline, |s| s.thickness_pt = Some(2.5));
        let ShapeKind::Text { spans, .. } = &app.doc().find_shape(id).expect("text").kind else {
            panic!("text");
        };
        assert_eq!(spans[0].underline_style.thickness_pt, Some(2.5));
        assert_eq!(spans[1].underline_style.thickness_pt, Some(2.5));
        assert!(spans[0].underline.is_on() && !spans[1].underline.is_on());
        assert_eq!(spans[1].underline_style.kind, LineKind::Single);
    }

    /// With no text selected the window opens on the plain look and does
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
