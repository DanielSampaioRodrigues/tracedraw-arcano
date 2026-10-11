//! The browser build's font folder: `fonts/fonts.json` next to the page
//! lists the families it holds and their files. The page fetches a
//! family's files the first time a drawing's text needs it (see
//! `web::fetch_requested_fonts`), so fonts cost nothing until used.

use std::collections::BTreeMap;

/// The folder's families and their file names, from `fonts.json`: an
/// object mapping each family to a list of file names. File names that
/// could leave the folder or are not font files are dropped, and so are
/// families left without files; anything else that does not read gives
/// an empty list.
pub fn parse_list(bytes: &[u8]) -> BTreeMap<String, Vec<String>> {
    let Ok(map) = serde_json::from_slice::<BTreeMap<String, serde_json::Value>>(bytes) else {
        return BTreeMap::new();
    };
    map.into_iter()
        .filter_map(|(family, files)| {
            let files: Vec<String> = files
                .as_array()?
                .iter()
                .filter_map(|f| f.as_str())
                .filter(|f| safe_file_name(f))
                .map(String::from)
                .collect();
            (!family.trim().is_empty() && !files.is_empty()).then_some((family, files))
        })
        .collect()
}

/// A plain font file name in the folder: letters, digits, `.`, `_` and
/// `-`, ending in a font extension, no path parts.
fn safe_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && [".ttf", ".otf", ".ttc"].iter().any(|e| lower.ends_with(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_font_list_reads_families_and_keeps_files_in_the_folder() {
        let list = parse_list(
            br#"{
                "Carlito": ["Carlito-Regular.ttf", "Carlito-Bold.ttf"],
                "Open Sans": ["OpenSans-VF.ttf", "../secret.ttf", "/etc/x.ttf",
                              "a/b.ttf", "notes.txt", ".hidden.ttf", 3],
                "Nothing Left": ["readme.md"],
                "Not a list": "Font.ttf",
                "": ["Empty-Name.ttf"]
            }"#,
        );
        assert_eq!(list.len(), 2, "{list:?}");
        assert_eq!(list["Carlito"], ["Carlito-Regular.ttf", "Carlito-Bold.ttf"]);
        assert_eq!(list["Open Sans"], ["OpenSans-VF.ttf"]);
        // Damaged lists give nothing, never a panic.
        assert!(parse_list(b"").is_empty());
        assert!(parse_list(b"[1, 2]").is_empty());
        assert!(parse_list(br#"{"A": ["A.ttf""#).is_empty());
    }
}
