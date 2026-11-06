//! A picture for every panel, for the column of panel buttons that
//! stands in for a folded panel group. Each is drawn on a 16-unit grid
//! in the icon colour, with one detail in the icons' accent so that the
//! column reads at a glance. Panels whose work is a tool's borrow that
//! tool's picture.

use crate::app::DockerTab;
use crate::theme::Tokens;
use crate::tools::Tool;
use egui::{epaint, vec2, Color32, Painter, Pos2, Rect, Stroke};

/// The grid: `r` split into 16 units, with the ink and accent colours.
struct G<'a> {
    p: &'a Painter,
    r: Rect,
    k: f32,
    ink: Color32,
    acc: Color32,
}

impl G<'_> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.r.min + vec2(x, y) * self.k
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.at(x0, y0), self.at(x1, y1))
    }
    fn s(&self, c: Color32) -> Stroke {
        Stroke::new(1.2 * self.k.max(0.75), c)
    }
    fn line(&self, pts: &[(f32, f32)], c: Color32) {
        let pts = pts.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.p.add(epaint::PathShape::line(pts, self.s(c)));
    }
    fn frame(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        self.p.rect_stroke(
            self.rect(x0, y0, x1, y1),
            1.5 * self.k,
            self.s(c),
            egui::StrokeKind::Middle,
        );
    }
    fn fill(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        self.p
            .rect_filled(self.rect(x0, y0, x1, y1), 1.2 * self.k, c);
    }
    fn ring(&self, x: f32, y: f32, rad: f32, c: Color32) {
        self.p.circle_stroke(self.at(x, y), rad * self.k, self.s(c));
    }
    fn dot(&self, x: f32, y: f32, rad: f32, c: Color32) {
        self.p.circle_filled(self.at(x, y), rad * self.k, c);
    }
    fn poly(&self, pts: &[(f32, f32)], fill: Color32, line: Color32) {
        let pts = pts.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.p
            .add(epaint::PathShape::convex_polygon(pts, fill, self.s(line)));
    }
    fn text(&self, x: f32, y: f32, t: &str, size: f32, c: Color32, bold: bool) {
        let font = if bold {
            crate::theme::bold(size * self.k)
        } else {
            egui::FontId::proportional(size * self.k)
        };
        self.p
            .text(self.at(x, y), egui::Align2::CENTER_CENTER, t, font, c);
    }
    /// A quarter of an ellipse-ish curve through three points.
    fn curve(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Color32) {
        let pts: Vec<Pos2> = (0..=12)
            .map(|i| {
                let t = i as f32 / 12.0;
                let u = 1.0 - t;
                let x = u * u * a.0 + 2.0 * u * t * b.0 + t * t * c.0;
                let y = u * u * a.1 + 2.0 * u * t * b.1 + t * t * c.1;
                self.at(x, y)
            })
            .collect();
        self.p.add(epaint::PathShape::line(pts, self.s(col)));
    }
}

/// Draw `tab`'s picture in `r`.
pub fn draw(p: &Painter, r: Rect, tab: DockerTab) {
    let ink = Tokens::icon();
    let tool = |t: Tool| crate::ui::icons::draw(p, r, t, ink);
    let g = G {
        p,
        r,
        k: r.width() / 16.0,
        ink,
        acc: crate::ui::icons::accent(),
    };
    let (i, a) = (g.ink, g.acc);
    let bg = Tokens::panel();
    match tab {
        // A sheet with ruled lines, the top one in the accent.
        DockerTab::Properties => {
            g.frame(2.5, 1.5, 13.5, 14.5, i);
            g.line(&[(5.0, 5.0), (11.0, 5.0)], a);
            g.line(&[(5.0, 8.0), (11.0, 8.0)], i);
            g.line(&[(5.0, 11.0), (9.0, 11.0)], i);
        }
        // Three stacked layers, the top one in the accent.
        DockerTab::Objects => {
            for (dy, top) in [(4.0, false), (0.0, false), (-4.0, true)] {
                let y = 8.0 + dy;
                g.poly(
                    &[(1.5, y), (8.0, y - 3.5), (14.5, y), (8.0, y + 3.5)],
                    if top { a } else { bg },
                    if top { a } else { i },
                );
            }
        }
        // A question mark in a round bubble.
        DockerTab::Hints => {
            g.ring(8.0, 8.0, 6.5, i);
            g.text(8.0, 8.2, "?", 10.0, a, true);
        }
        DockerTab::Transformations => tool(Tool::FreeTransform),
        DockerTab::Undo => crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Undo, ink),
        // Bars lined up on a line in the accent.
        DockerTab::AlignDistribute => {
            g.line(&[(2.0, 1.5), (2.0, 14.5)], a);
            g.fill(3.5, 3.0, 13.5, 6.5, i);
            g.fill(3.5, 9.5, 9.5, 13.0, i);
        }
        // Two shapes overlapping, their common part in the accent.
        DockerTab::Shaping => {
            g.frame(1.5, 1.5, 10.0, 10.0, i);
            g.fill(6.0, 6.0, 10.0, 10.0, a);
            g.ring(10.0, 10.0, 4.5, i);
        }
        // Three copies stepping down, the last in the accent.
        DockerTab::StepAndRepeat => {
            g.frame(1.5, 1.5, 6.5, 6.5, i);
            g.frame(5.5, 5.5, 10.5, 10.5, i);
            g.fill(9.5, 9.5, 14.5, 14.5, a);
        }
        DockerTab::Text => tool(Tool::Text),
        // A glyph in a cell.
        DockerTab::Glyphs => {
            g.frame(1.5, 1.5, 14.5, 14.5, i);
            g.text(8.0, 8.3, "&", 11.0, a, true);
        }
        // Three overlapping discs in print colours.
        DockerTab::Color => {
            let c = |r, g2, b| Color32::from_rgb(r, g2, b);
            g.dot(6.0, 6.0, 4.0, c(0x00, 0xAE, 0xEF));
            g.dot(10.0, 6.0, 4.0, c(0xEC, 0x00, 0x8C));
            g.dot(8.0, 10.0, 4.0, c(0xFF, 0xD5, 0x00));
            g.ring(8.0, 8.0, 7.0, i);
        }
        // Swatches in a style frame.
        DockerTab::ColorStyles => {
            g.fill(1.5, 2.0, 6.5, 7.0, a);
            g.fill(9.5, 2.0, 14.5, 7.0, i);
            g.frame(1.5, 9.0, 6.5, 14.0, i);
            g.frame(9.5, 9.0, 14.5, 14.0, i);
        }
        // A shape with a style tag.
        DockerTab::ObjectStyles => {
            g.frame(1.5, 4.5, 11.5, 14.5, i);
            g.ring(6.5, 9.5, 2.5, i);
            g.fill(9.5, 1.5, 14.5, 6.5, a);
        }
        // A magnifier over lines of text.
        DockerTab::FindReplace => {
            g.line(&[(1.5, 2.5), (9.0, 2.5)], i);
            g.line(&[(1.5, 5.5), (6.0, 5.5)], i);
            g.ring(9.0, 9.0, 3.5, a);
            g.line(&[(11.5, 11.5), (14.5, 14.5)], i);
        }
        // A code bracket with an accent slash.
        DockerTab::Scripts => {
            g.line(&[(5.0, 3.5), (1.5, 8.0), (5.0, 12.5)], i);
            g.line(&[(11.0, 3.5), (14.5, 8.0), (11.0, 12.5)], i);
            g.line(&[(9.5, 2.5), (6.5, 13.5)], a);
        }
        // A column of swatches.
        DockerTab::Palettes => {
            for (n, y) in [1.5f32, 6.0, 10.5].into_iter().enumerate() {
                g.fill(2.0, y, 7.0, y + 4.0, if n == 0 { a } else { i });
                g.frame(9.0, y + 0.5, 14.0, y + 3.5, i);
            }
        }
        // A lens over a shape.
        DockerTab::Lens => {
            g.frame(1.5, 6.5, 9.5, 14.5, i);
            g.p.circle_filled(
                g.at(10.0, 6.0),
                4.5 * g.k,
                Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 90),
            );
            g.ring(10.0, 6.0, 4.5, a);
        }
        DockerTab::Blend => tool(Tool::Blend),
        DockerTab::Contour => tool(Tool::Contour),
        DockerTab::Envelope => tool(Tool::Envelope),
        DockerTab::Extrude => tool(Tool::Extrude),
        // A raised face with bevelled edges.
        DockerTab::Bevel => {
            g.frame(1.5, 1.5, 14.5, 14.5, i);
            g.fill(5.0, 5.0, 11.0, 11.0, a);
            for (x0, y0, x1, y1) in [
                (1.5, 1.5, 5.0, 5.0),
                (14.5, 1.5, 11.0, 5.0),
                (1.5, 14.5, 5.0, 11.0),
                (14.5, 14.5, 11.0, 11.0),
            ] {
                g.line(&[(x0, y0), (x1, y1)], i);
            }
        }
        DockerTab::BrushStrokes => tool(Tool::BrushStrokes),
        // A picture with a dashed mask frame.
        DockerTab::BitmapMask => {
            g.frame(1.5, 2.5, 14.5, 13.5, i);
            g.line(
                &[
                    (3.0, 12.0),
                    (7.0, 7.0),
                    (10.0, 10.0),
                    (11.5, 8.5),
                    (13.0, 12.0),
                ],
                i,
            );
            g.dot(11.0, 5.5, 1.6, a);
        }
        // A table with an accent header row.
        DockerTab::ObjectData => {
            g.fill(1.5, 2.0, 14.5, 5.0, a);
            g.frame(1.5, 2.0, 14.5, 14.0, i);
            g.line(&[(1.5, 9.5), (14.5, 9.5)], i);
            g.line(&[(7.0, 5.0), (7.0, 14.0)], i);
        }
        // Two links of a chain.
        DockerTab::Links => {
            g.p.rect_stroke(
                g.rect(1.0, 5.0, 9.5, 11.0),
                3.0 * g.k,
                g.s(i),
                egui::StrokeKind::Middle,
            );
            g.p.rect_stroke(
                g.rect(6.5, 5.0, 15.0, 11.0),
                3.0 * g.k,
                g.s(a),
                egui::StrokeKind::Middle,
            );
        }
        // A star in a dashed frame: one drawing, used many times.
        DockerTab::Symbols => {
            let mut pts = Vec::new();
            for n in 0..10 {
                let ang = -std::f32::consts::FRAC_PI_2 + n as f32 * std::f32::consts::PI / 5.0;
                let rad = if n % 2 == 0 { 6.0 } else { 2.6 };
                pts.push(g.at(8.0 + rad * ang.cos(), 8.5 + rad * ang.sin()));
            }
            g.p.add(epaint::PathShape::closed_line(pts, g.s(a)));
            g.line(&[(1.0, 1.0), (3.0, 1.0)], i);
            g.line(&[(13.0, 1.0), (15.0, 1.0)], i);
            g.line(&[(1.0, 15.0), (3.0, 15.0)], i);
            g.line(&[(13.0, 15.0), (15.0, 15.0)], i);
        }
        // Two pages, the front one with an accent corner.
        DockerTab::Pages => {
            g.frame(1.5, 1.5, 9.5, 12.0, i);
            g.poly(
                &[
                    (5.5, 4.0),
                    (11.5, 4.0),
                    (14.5, 7.0),
                    (14.5, 14.5),
                    (5.5, 14.5),
                ],
                bg,
                i,
            );
            g.poly(&[(11.5, 4.0), (14.5, 7.0), (11.5, 7.0)], a, a);
        }
        // Guides crossing over a ruler edge.
        DockerTab::Guidelines => {
            g.line(&[(1.5, 1.5), (14.5, 1.5)], i);
            g.line(&[(1.5, 1.5), (1.5, 14.5)], i);
            for x in [4.0, 7.0, 10.0, 13.0] {
                g.line(&[(x, 1.5), (x, 3.0)], i);
            }
            g.line(&[(9.0, 4.0), (9.0, 14.5)], a);
            g.line(&[(3.5, 9.0), (14.5, 9.0)], a);
        }
        // "Aa", the small one in the accent.
        DockerTab::Fonts => {
            g.text(5.5, 8.5, "A", 13.0, i, true);
            g.text(11.8, 9.8, "a", 10.0, a, true);
        }
        // Sample lines of type in falling sizes.
        DockerTab::FontSampler => {
            g.text(4.5, 4.5, "Ag", 8.0, a, true);
            g.line(&[(9.0, 4.5), (14.5, 4.5)], i);
            g.line(&[(1.5, 9.0), (14.5, 9.0)], i);
            g.line(&[(1.5, 12.5), (11.0, 12.5)], i);
        }
        // A rounded corner.
        DockerTab::Corners => {
            g.line(&[(2.0, 14.5), (2.0, 8.0)], i);
            g.curve((2.0, 8.0), (2.0, 2.0), (8.0, 2.0), a);
            g.line(&[(8.0, 2.0), (14.5, 2.0)], i);
        }
        // Two curves meeting at an accent node.
        DockerTab::JoinCurves => {
            g.curve((1.5, 13.0), (3.0, 4.0), (7.5, 8.0), i);
            g.curve((8.5, 8.0), (13.0, 12.0), (14.5, 3.0), i);
            g.fill(6.5, 6.5, 9.5, 9.5, a);
        }
        // Letters riding an arc.
        DockerTab::FitToPath => {
            g.curve((1.5, 14.0), (8.0, 0.0), (14.5, 14.0), i);
            g.dot(4.0, 8.0, 1.3, a);
            g.dot(8.0, 6.0, 1.3, a);
            g.dot(12.0, 8.0, 1.3, a);
        }
        // Crosshairs on a point.
        DockerTab::Coordinates => {
            g.line(&[(8.0, 1.0), (8.0, 15.0)], i);
            g.line(&[(1.0, 8.0), (15.0, 8.0)], i);
            g.ring(8.0, 8.0, 3.5, a);
        }
        // A grid of tiles, some in the accent.
        DockerTab::VectorMosaic | DockerTab::PictureMosaic => {
            for row in 0..3 {
                for col in 0..3 {
                    let (x, y) = (1.5 + col as f32 * 4.5, 1.5 + row as f32 * 4.5);
                    let on = (row + col) % 2 == 0;
                    let pict = tab == DockerTab::PictureMosaic;
                    if on != pict {
                        g.fill(x, y, x + 3.8, y + 3.8, a);
                    } else {
                        g.frame(x + 0.4, y + 0.4, x + 3.4, y + 3.4, i);
                    }
                }
            }
        }
        // A tray holding an item.
        DockerTab::Shelf => {
            g.line(&[(1.5, 8.0), (1.5, 14.0), (14.5, 14.0), (14.5, 8.0)], i);
            g.fill(4.5, 3.0, 11.5, 10.5, a);
        }
        DockerTab::Export => {
            crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Export, ink)
        }
        // A speech bubble with lines.
        DockerTab::Comments => {
            g.poly(
                &[
                    (1.5, 2.0),
                    (14.5, 2.0),
                    (14.5, 11.0),
                    (6.0, 11.0),
                    (3.0, 14.5),
                    (3.5, 11.0),
                    (1.5, 11.0),
                ],
                bg,
                i,
            );
            g.line(&[(4.0, 5.0), (12.0, 5.0)], a);
            g.line(&[(4.0, 8.0), (10.0, 8.0)], i);
        }
        // Sparkles.
        DockerTab::AiGenerate => {
            sparkle(&g, 6.5, 9.0, 5.5, a);
            sparkle(&g, 12.5, 3.5, 2.8, i);
        }
        // "fx".
        DockerTab::Effects => {
            g.text(5.5, 8.0, "f", 13.0, i, true);
            g.text(11.0, 9.5, "x", 11.0, a, true);
        }
        // A play button: run a recorded macro.
        DockerTab::Macros => {
            g.ring(8.0, 8.0, 6.5, i);
            g.poly(&[(6.3, 5.0), (11.3, 8.0), (6.3, 11.0)], a, a);
        }
        // A page fed by rows of records.
        DockerTab::PrintMerge => {
            g.frame(6.5, 1.5, 14.5, 14.5, i);
            for y in [3.5, 7.0, 10.5] {
                g.fill(1.5, y, 5.0, y + 2.0, a);
            }
            g.line(&[(8.5, 5.0), (12.5, 5.0)], i);
            g.line(&[(8.5, 8.0), (12.5, 8.0)], i);
        }
        // A bubble with a sparkle: the assistant.
        DockerTab::Assistente => {
            g.poly(
                &[
                    (1.5, 2.0),
                    (14.5, 2.0),
                    (14.5, 11.0),
                    (6.0, 11.0),
                    (3.0, 14.5),
                    (3.5, 11.0),
                    (1.5, 11.0),
                ],
                bg,
                i,
            );
            sparkle(&g, 8.0, 6.5, 3.5, a);
        }
        // A pilcrow.
        DockerTab::TextStyles => {
            g.text(8.0, 8.3, "¶", 14.0, a, true);
        }
        // A letter over an accent baseline.
        DockerTab::Character => {
            g.text(8.0, 7.5, "A", 13.0, i, true);
            g.line(&[(2.0, 14.0), (14.0, 14.0)], a);
        }
        // Lines of a justified paragraph, its first line indented.
        DockerTab::Paragraph => {
            g.line(&[(5.0, 2.5), (14.5, 2.5)], a);
            for y in [6.5, 10.5] {
                g.line(&[(1.5, y), (14.5, y)], i);
            }
            g.line(&[(1.5, 14.5), (9.0, 14.5)], i);
        }
        // An open book with a check: the house's own pass over the text.
        DockerTab::HousePress => {
            g.curve((8.0, 4.0), (4.5, 2.0), (1.0, 3.0), i);
            g.line(&[(1.0, 3.0), (1.0, 13.0)], i);
            g.curve((1.0, 13.0), (4.5, 12.0), (8.0, 14.0), i);
            g.curve((8.0, 4.0), (11.5, 2.0), (15.0, 3.0), i);
            g.line(&[(15.0, 3.0), (15.0, 13.0)], i);
            g.curve((15.0, 13.0), (11.5, 12.0), (8.0, 14.0), i);
            g.line(&[(8.0, 4.0), (8.0, 14.0)], i);
            g.line(&[(9.5, 8.5), (11.0, 10.0), (13.5, 6.5)], a);
        }
    }
}

/// A four-pointed sparkle centred on (x, y): two thin diamonds crossed.
fn sparkle(g: &G, x: f32, y: f32, rad: f32, c: Color32) {
    let w = rad * 0.3;
    g.poly(&[(x, y - rad), (x + w, y), (x, y + rad), (x - w, y)], c, c);
    g.poly(&[(x - rad, y), (x, y - w), (x + rad, y), (x, y + w)], c, c);
}

/// The picture as a plain function, for the places that take one.
pub fn painter_for(tab: DockerTab) -> impl Fn(&Painter, Rect) {
    move |p, r| draw(p, r, tab)
}
