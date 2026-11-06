//! Page > Sections: parts of the drawing with their own page numbering
//! (roman in the front matter, arabic from the first chapter) and a title
//! that running heads show (`tracedraw_core::document::Section`). Page >
//! Insert Running Head puts a field showing the section's title on the
//! active master layer (or on every page). See docs/behavior/sections.md.

use crate::app::App;
use tracedraw_core::document::{PageNumberField, Section};
use tracedraw_core::Command;

/// The numbering styles, in [`Section::style`]'s order (i18n keys).
const STYLES: [&str; 5] = [
    "section.style_arabic",
    "section.style_upper_roman",
    "section.style_lower_roman",
    "section.style_upper_alpha",
    "section.style_lower_alpha",
];

impl App {
    /// Replace the drawing's sections (one undo step).
    pub fn set_sections(&mut self, sections: Vec<Section>) {
        if sections != self.doc().sections {
            self.run(Command::SetSections { sections });
        }
    }

    /// Page > Insert Running Head: a field showing the title of the
    /// page's section, at the top centre, where page numbers go (the
    /// active master layer, else each page).
    pub fn insert_running_head(&mut self) {
        let keep = self.page_numbers.position;
        self.page_numbers.position = 2;
        self.insert_page_field(crate::ops2::PageNumberWhere::All, 1);
        self.page_numbers.position = keep;
    }
}

/// The Sections window.
pub fn dialog(app: &mut App, ctx: &egui::Context, close: &mut bool) {
    use crate::i18n::tr;
    use crate::ui::tips::{pair, RichTip as _};
    let pages: Vec<tracedraw_core::PageId> = app.doc().pages.iter().map(|p| p.id).collect();
    let mut sections = app.doc().sections.clone();
    sections.retain(|s| pages.contains(&s.first_page));
    sections.sort_by_key(|s| pages.iter().position(|p| *p == s.first_page));
    let before = sections.clone();
    crate::ui::dialogs::window(ctx, tr("section.title")).show(ctx, |ui| {
        ui.label(egui::RichText::new(tr("section.intro")).color(crate::theme::Tokens::text_dim()));
        ui.add_space(6.0);
        let mut remove = None;
        egui::Grid::new("sections")
            .num_columns(5)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.strong(tr("section.first_page"));
                ui.strong(tr("section.numbering"));
                ui.strong(tr("section.start_at"));
                ui.strong(tr("section.name"));
                ui.label("");
                ui.end_row();
                for (i, s) in sections.iter_mut().enumerate() {
                    let mut n = pages.iter().position(|p| *p == s.first_page).unwrap_or(0) + 1;
                    ui.add(
                        egui::DragValue::new(&mut n)
                            .range(1..=pages.len())
                            .speed(0.1),
                    )
                    .rich_tip(pair("section.first_page", "section.first_page_desc"));
                    s.first_page = pages[n.clamp(1, pages.len()) - 1];
                    crate::ui::combo::from_id_salt(("section_style", i))
                        .selected_text(tr(STYLES[s.style.min(4) as usize]))
                        .show_ui(ui, |ui| {
                            for (k, key) in STYLES.iter().enumerate() {
                                ui.selectable_value(&mut s.style, k as u8, tr(key));
                            }
                        })
                        .response
                        .rich_tip(pair("section.numbering", "section.numbering_desc"));
                    ui.add(
                        egui::DragValue::new(&mut s.start_at)
                            .range(1..=9999)
                            .speed(0.1),
                    )
                    .rich_tip(pair("section.start_at", "section.start_at_desc"));
                    ui.add_sized([160.0, 20.0], egui::TextEdit::singleline(&mut s.title))
                        .rich_tip(pair("section.name", "section.name_desc"));
                    if ui
                        .small_button(tr("section.remove"))
                        .rich_tip(pair("section.remove", "section.remove_desc"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = remove {
            sections.remove(i);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .button(tr("section.add"))
                .rich_tip(pair("section.add", "section.add_desc"))
                .clicked()
            {
                let page = app.page;
                if !sections.iter().any(|s| s.first_page == page) {
                    sections.push(Section {
                        first_page: page,
                        style: 0,
                        start_at: 1,
                        title: String::new(),
                    });
                }
            }
            if ui
                .button(tr("section.book"))
                .rich_tip(pair("section.book", "section.book_desc"))
                .clicked()
            {
                // Front matter in lower roman from page 1, the text in
                // arabic from the active page.
                sections = vec![Section {
                    first_page: pages[0],
                    style: 2,
                    start_at: 1,
                    title: String::new(),
                }];
                if app.page != pages[0] {
                    sections.push(Section {
                        first_page: app.page,
                        style: 0,
                        start_at: 1,
                        title: String::new(),
                    });
                }
            }
            if ui
                .button(tr("section.insert_head"))
                .rich_tip(pair("section.insert_head", "section.insert_head_desc"))
                .clicked()
            {
                app.insert_running_head();
            }
        });
        ui.separator();
        if ui
            .button(tr("dialog.close"))
            .rich_tip(pair("dialog.close", "section.close_desc"))
            .clicked()
        {
            *close = true;
        }
    });
    if sections != before {
        app.set_sections(sections);
    }
}

/// What a field shows on page `index` (for the preview in tests).
pub fn shown(app: &App, f: &PageNumberField, index: usize) -> String {
    app.doc().field_text(f, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_number_their_pages_and_name_the_running_head() {
        let mut app = App::headless();
        for _ in 0..5 {
            app.add_page();
        }
        let pages: Vec<_> = app.doc().pages.iter().map(|p| p.id).collect();
        assert_eq!(pages.len(), 6);
        app.set_sections(vec![
            Section {
                first_page: pages[0],
                style: 2,
                start_at: 1,
                title: "Apresentação".into(),
            },
            Section {
                first_page: pages[3],
                style: 0,
                start_at: 1,
                title: "Capítulo 1".into(),
            },
        ]);
        let num = PageNumberField {
            start_at: 1,
            style: 0,
            prefix: String::new(),
            suffix: String::new(),
            show: 0,
        };
        let labels: Vec<String> = (0..6).map(|i| shown(&app, &num, i)).collect();
        assert_eq!(labels, ["i", "ii", "iii", "1", "2", "3"]);
        let head = PageNumberField {
            show: 1,
            ..num.clone()
        };
        assert_eq!(shown(&app, &head, 1), "Apresentação");
        assert_eq!(shown(&app, &head, 4), "Capítulo 1");
        // Without sections the field numbers as before.
        app.set_sections(Vec::new());
        assert_eq!(shown(&app, &num, 4), "5");
        // One undo step each.
        app.undo();
        assert_eq!(app.doc().sections.len(), 2);
    }
}
