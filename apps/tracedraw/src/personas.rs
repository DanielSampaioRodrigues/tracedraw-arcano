//! Arcano edition: workspaces one click apart, at the right of the menu
//! bar: Drawing and Page. Each keeps its own panels: leaving one keeps
//! how its panels were, and coming back puts them back; the first time,
//! Page lays out the panels a book's pages need (Pages and Layers;
//! Character, Paragraph and Text Styles; House Press and Align). The
//! drawing, the tools and the menus stay the same. See
//! docs/behavior/personas.md.

use crate::app::{App, DockerTab};
use crate::settings::{DockerSetup, StackGroupSetup};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persona {
    Drawing,
    Page,
}

impl Persona {
    pub const ALL: [Persona; 2] = [Persona::Drawing, Persona::Page];

    pub fn id(self) -> &'static str {
        match self {
            Persona::Drawing => "drawing",
            Persona::Page => "page",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Persona::Drawing => "persona.drawing",
            Persona::Page => "persona.page",
        }
    }

    pub fn from_id(id: &str) -> Persona {
        match id {
            "page" => Persona::Page,
            _ => Persona::Drawing,
        }
    }

    /// The panels it starts with.
    pub fn preset(self) -> DockerSetup {
        let k = |t: DockerTab| t.key().to_string();
        let group = |tabs: &[DockerTab], weight: f32| StackGroupSetup {
            tabs: tabs.iter().map(|t| k(*t)).collect(),
            front: k(tabs[0]),
            weight,
        };
        let groups: Vec<StackGroupSetup> = match self {
            Persona::Drawing => vec![
                group(&[DockerTab::Properties, DockerTab::Hints], 1.4),
                group(&[DockerTab::Objects], 1.0),
            ],
            Persona::Page => vec![
                group(&[DockerTab::Pages, DockerTab::Objects], 1.0),
                group(
                    &[
                        DockerTab::Character,
                        DockerTab::Paragraph,
                        DockerTab::TextStyles,
                    ],
                    1.5,
                ),
                group(&[DockerTab::HousePress, DockerTab::AlignDistribute], 1.0),
            ],
        };
        let open: Vec<String> = groups.iter().flat_map(|g| g.tabs.clone()).collect();
        DockerSetup {
            front: open[0].clone(),
            open,
            stacks: vec![groups],
            ..Default::default()
        }
    }
}

impl App {
    /// The workspace in use.
    pub fn persona(&self) -> Persona {
        Persona::from_id(&self.settings.arcano_persona)
    }

    /// Change to `to`: the panels of the one left are kept, and `to`'s
    /// come back (its preset the first time).
    pub fn switch_persona(&mut self, to: Persona) {
        let from = self.persona();
        if from == to {
            return;
        }
        let now = self.docker_setup();
        self.settings
            .arcano_persona_layouts
            .insert(from.id().to_string(), now);
        let setup = self
            .settings
            .arcano_persona_layouts
            .get(to.id())
            .cloned()
            .unwrap_or_else(|| to.preset());
        self.set_docker_setup(&setup);
        self.show_dockers = true;
        if to == Persona::Page {
            self.show_rulers = true;
            self.show_guides = true;
        }
        self.stack_flyout = None;
        self.settings.arcano_persona = to.id().to_string();
        self.status = crate::i18n::trf("persona.switched", &[("name", &crate::i18n::tr(to.key()))]);
    }

    /// Back to the panels `persona` starts with.
    pub fn reset_persona(&mut self) {
        let p = self.persona();
        self.settings.arcano_persona_layouts.remove(p.id());
        self.set_docker_setup(&p.preset());
    }
}

/// The switch at the right of the menu bar: one button per workspace.
pub fn switch(app: &mut App, ui: &mut egui::Ui) {
    use crate::i18n::tr;
    use crate::theme::Tokens;
    use crate::ui::tips::RichTip as _;
    let now = app.persona();
    let frame = egui::Frame::new()
        .fill(Tokens::field())
        .corner_radius(7.0)
        .inner_margin(egui::Margin::same(2));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // Right to left: the last first.
            for p in Persona::ALL.iter().rev() {
                let on = *p == now;
                let label = tr(p.key());
                let galley = ui.painter().layout_no_wrap(
                    label,
                    egui::FontId::proportional(12.0),
                    Tokens::text(),
                );
                let size = egui::vec2(galley.size().x + 20.0, 20.0);
                let (r, resp) = ui.allocate_exact_size(size, egui::Sense::click());
                if on {
                    ui.painter().rect_filled(r, 5.0, Tokens::panel());
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 5.0, Tokens::tool_hover());
                }
                if on {
                    ui.painter().hline(
                        r.shrink2(egui::vec2(8.0, 0.0)).x_range(),
                        r.bottom() - 1.5,
                        egui::Stroke::new(2.0, crate::ui::icons::accent()),
                    );
                }
                ui.painter().galley(
                    egui::pos2(r.left() + 10.0, r.center().y - galley.size().y / 2.0),
                    galley,
                    if on {
                        Tokens::text()
                    } else {
                        Tokens::text_dim()
                    },
                );
                let desc = format!("{}_desc", p.key());
                if resp
                    .rich_tip(crate::ui::tips::pair(p.key(), &desc))
                    .clicked()
                {
                    app.switch_persona(*p);
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_workspace_keeps_its_panels() {
        let mut app = App::headless();
        assert_eq!(app.persona(), Persona::Drawing);
        app.open_docker(DockerTab::Color);
        app.switch_persona(Persona::Page);
        assert_eq!(app.persona(), Persona::Page);
        // The page's panels, in their groups.
        for t in [
            DockerTab::Pages,
            DockerTab::Character,
            DockerTab::Paragraph,
            DockerTab::TextStyles,
            DockerTab::HousePress,
        ] {
            assert!(app.docker_open(t), "{t:?}");
        }
        assert!(!app.docker_open(DockerTab::Color));
        // Back: the drawing's panels as they were left.
        app.switch_persona(Persona::Drawing);
        assert!(app.docker_open(DockerTab::Color));
        assert!(!app.docker_open(DockerTab::Character));
        // A change kept for the next time.
        app.switch_persona(Persona::Page);
        app.open_docker(DockerTab::Glyphs);
        app.switch_persona(Persona::Drawing);
        app.switch_persona(Persona::Page);
        assert!(app.docker_open(DockerTab::Glyphs));
        // Reset: the preset again.
        app.reset_persona();
        assert!(!app.docker_open(DockerTab::Glyphs));
    }
}
