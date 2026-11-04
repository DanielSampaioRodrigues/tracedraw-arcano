//! Arcano edition: the house library. The Shelf (Window > Panels >
//! Shelf, "Biblioteca" in Portuguese) can be kept in a folder that the
//! editora's computers sync (Google Drive, OneDrive, iCloud...): the
//! objects and colours anyone adds there show on every computer, and the
//! drawings saved as house templates (its `Modelos` folder) show in File >
//! New From Template. A change made on another computer is read within a
//! couple of seconds. See docs/behavior/house-library.md.
//!
//! Inside the chosen folder: `Biblioteca TraceDraw/biblioteca.json` (a
//! shelf file) and `Biblioteca TraceDraw/Modelos/*.tdt`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The folder made inside the one chosen.
pub const FOLDER: &str = "Biblioteca TraceDraw";
/// The shelf file in it.
pub const FILE: &str = "biblioteca.json";
/// The house templates' folder in it.
pub const TEMPLATES: &str = "Modelos";

static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Follow the folder the preferences name (empty: none, the shelf stays
/// in the preferences folder).
pub fn set(chosen: &str) {
    let dir = (!chosen.trim().is_empty()).then(|| Path::new(chosen.trim()).join(FOLDER));
    if let Ok(mut d) = DIR.lock() {
        *d = dir;
    }
}

/// The library's folder (`<chosen>/Biblioteca TraceDraw`), when one is
/// chosen.
pub fn dir() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    DIR.lock().ok().and_then(|d| d.clone())
}

/// The shelf file in the library folder.
pub fn shelf_file() -> Option<PathBuf> {
    dir().map(|d| d.join(FILE))
}

/// The house templates' folder.
pub fn templates_dir() -> Option<PathBuf> {
    dir().map(|d| d.join(TEMPLATES))
}

/// A file name for `title` in `dir` that is not taken: "Name.tdt",
/// "Name 2.tdt"...
pub fn free_name(dir: &Path, title: &str, ext: &str) -> PathBuf {
    let clean: String = title
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { ' ' } else { c })
        .collect();
    let base = match clean.trim() {
        "" => "Modelo".to_string(),
        t => t.to_string(),
    };
    let mut p = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base} {n}.{ext}"));
        n += 1;
    }
    p
}

impl crate::app::App {
    /// The library follows the folder `chosen` (empty: this computer
    /// only). The shelf is read again from there; what was on this
    /// computer's shelf goes with it when the folder's library is new.
    pub fn set_library_folder(&mut self, chosen: String) {
        let before = std::mem::take(&mut self.shelf.items);
        self.settings.arcano_library = chosen.clone();
        set(&chosen);
        if let Some(d) = dir() {
            let _ = std::fs::create_dir_all(d.join(TEMPLATES));
        }
        let fresh = shelf_file().is_some_and(|f| !f.exists());
        self.shelf.reload();
        if fresh && !before.is_empty() {
            for item in before {
                self.shelf.push(item);
            }
            self.shelf.save();
        }
        self.status = match dir() {
            Some(d) => crate::i18n::trf("library.now_in", &[("p", &d.display().to_string())]),
            None => crate::i18n::tr("library.now_local"),
        };
    }

    /// Save the drawing as a house template, in the library's `Modelos`
    /// folder, named after the drawing.
    pub fn save_house_template(&mut self) {
        let Some(dir) = templates_dir() else {
            self.status = crate::i18n::tr("library.no_folder");
            return;
        };
        let _ = std::fs::create_dir_all(&dir);
        let path = free_name(&dir, &self.engine.document().title, "tdt");
        let written = tracedraw_text::fonts()
            .with_carried_fonts(self.engine.document())
            .to_json()
            .map_err(|e| e.to_string())
            .and_then(|json| crate::files::write(&path, json).map_err(|e| e.to_string()));
        self.status = match written {
            Ok(()) => crate::i18n::trf(
                "library.template_saved",
                &[("name", &path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())],
            ),
            Err(e) => crate::i18n::trf("status.save_failed", &[("e", &e)]),
        };
    }
}

/// The library row at the top of the Shelf docker: where the library is,
/// Choose Folder, This Computer Only, and Save as House Template.
pub fn header(app: &mut crate::app::App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, trf};
    use crate::ui::tips::RichTip as _;
    let shown = match dir() {
        Some(d) => trf("library.in_folder", &[("p", &d.display().to_string())]),
        None => tr("library.local"),
    };
    ui.label(egui::RichText::new(shown).color(crate::theme::Tokens::text_dim()))
        .rich_tip(crate::ui::tips::pair("library.where", "library.where_desc"));
    ui.horizontal_wrapped(|ui| {
        let action = crate::files::FolderUse::HouseLibrary.action();
        let can = matches!(action, crate::files::FolderAction::PickFolder(_));
        let tip = crate::files::FolderUse::HouseLibrary.tip().unwrap_or("library.choose_desc");
        if ui
            .add_enabled(can, egui::Button::new(tr("library.choose")))
            .rich_tip(crate::ui::tips::pair("library.choose", tip))
            .clicked()
        {
            if let crate::files::FolderAction::PickFolder(token) = action {
                if let Some(p) = crate::files::pick_folder(token) {
                    app.set_library_folder(p.display().to_string());
                }
            }
        }
        if dir().is_some()
            && ui
                .button(tr("library.local_only"))
                .rich_tip(crate::ui::tips::pair("library.local_only", "library.local_only_desc"))
                .clicked()
        {
            app.set_library_folder(String::new());
        }
        let drawing = app.has_document() && !app.show_welcome;
        if ui
            .add_enabled(drawing && dir().is_some(), egui::Button::new(tr("library.save_template")))
            .rich_tip(crate::ui::tips::pair("library.save_template", "library.save_template_desc"))
            .clicked()
        {
            app.save_house_template();
        }
    });
    ui.separator();
    app.shelf.follow_file();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_names_are_free_and_clean() {
        let d = std::env::temp_dir().join(format!("arcano-lib-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        let a = free_name(&d, "Capa: livro/1", "tdt");
        assert_eq!(a.file_name().unwrap().to_string_lossy(), "Capa  livro 1.tdt");
        std::fs::write(&a, "x").unwrap();
        let b = free_name(&d, "Capa: livro/1", "tdt");
        assert_eq!(b.file_name().unwrap().to_string_lossy(), "Capa  livro 1 2.tdt");
        assert_eq!(free_name(&d, "  ", "tdt").file_name().unwrap().to_string_lossy(), "Modelo.tdt");
        let _ = std::fs::remove_dir_all(&d);
    }
}
