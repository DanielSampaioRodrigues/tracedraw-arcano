//! The system's printers for the Print dialog's Printer list: the
//! printers CUPS knows (Linux and macOS), listed by `lpstat` and printed
//! to with `lp`, which takes the print PDF as it is. Where there is no
//! CUPS (Windows, the browser) the list is empty and the print goes
//! through the system PDF viewer, as before. See
//! docs/behavior/printing.md.

use std::path::Path;

/// A printer the system has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printer {
    /// The name `lp -d` takes.
    pub name: String,
    /// The system's default destination.
    pub default: bool,
}

/// The printers `lpstat -p -d` lists ("printer NAME is idle..." lines
/// and "system default destination: NAME"), the default one first.
pub fn parse_lpstat(output: &str) -> Vec<Printer> {
    let mut out: Vec<Printer> = Vec::new();
    let mut default: Option<String> = None;
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("printer ") {
            let name = rest.split_whitespace().next().unwrap_or_default();
            if !name.is_empty() && !out.iter().any(|p| p.name == name) {
                out.push(Printer {
                    name: name.to_string(),
                    default: false,
                });
            }
        } else if let Some(rest) = line.strip_prefix("system default destination:") {
            let name = rest.trim();
            if !name.is_empty() {
                default = Some(name.to_string());
            }
        }
    }
    if let Some(name) = default {
        if let Some(i) = out.iter().position(|p| p.name == name) {
            out[i].default = true;
            let p = out.remove(i);
            out.insert(0, p);
        }
    }
    out
}

/// The printers the system has, the default one first; empty where
/// none is set up or there is no CUPS.
#[cfg(not(any(target_arch = "wasm32", target_os = "windows")))]
pub fn system_printers() -> Vec<Printer> {
    let out = std::process::Command::new("lpstat")
        .args(["-p", "-d"])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output();
    match out {
        Ok(o) => parse_lpstat(&String::from_utf8_lossy(&o.stdout)),
        Err(e) => {
            log::info!("no printer list: lpstat: {e}");
            Vec::new()
        }
    }
}

#[cfg(any(target_arch = "wasm32", target_os = "windows"))]
pub fn system_printers() -> Vec<Printer> {
    Vec::new()
}

/// The printer the dialog starts on: the one remembered when the system
/// still has it, else the system's default (the first listed), else none
/// (the print goes through the PDF viewer).
pub fn starting_printer(remembered: Option<&str>, printers: &[Printer]) -> Option<String> {
    if let Some(r) = remembered {
        if printers.iter().any(|p| p.name == r) {
            return Some(r.to_string());
        }
    }
    printers.first().map(|p| p.name.clone())
}

/// The `lp` command that prints `pdf` on `printer`: `copies` copies,
/// collated as whole sets or sheet by sheet, under `title` in the queue.
pub fn lp_args(printer: &str, pdf: &Path, copies: u32, collate: bool, title: &str) -> Vec<String> {
    let mut args = vec!["-d".to_string(), printer.to_string()];
    if copies > 1 {
        args.push("-n".into());
        args.push(copies.min(999).to_string());
        args.push("-o".into());
        args.push(format!(
            "collate={}",
            if collate { "true" } else { "false" }
        ));
    }
    let title = title.trim();
    if !title.is_empty() {
        args.push("-t".into());
        args.push(title.to_string());
    }
    args.push("--".into());
    args.push(pdf.display().to_string());
    args
}

/// Send `pdf` to `printer` through `lp`. Ok with what `lp` said (the
/// request id), Err with its complaint.
#[cfg(not(any(target_arch = "wasm32", target_os = "windows")))]
pub fn print_pdf(
    printer: &str,
    pdf: &Path,
    copies: u32,
    collate: bool,
    title: &str,
) -> Result<String, String> {
    let out = std::process::Command::new("lp")
        .args(lp_args(printer, pdf, copies, collate, title))
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("lp: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("lp: {}", out.status)
        } else {
            err
        })
    }
}

#[cfg(any(target_arch = "wasm32", target_os = "windows"))]
pub fn print_pdf(
    printer: &str,
    _pdf: &Path,
    _copies: u32,
    _collate: bool,
    _title: &str,
) -> Result<String, String> {
    Err(format!("no printer {printer} here"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Printer list comes from `lpstat`: every printer, the default
    /// first, whatever the rest of the line says (#300).
    #[test]
    fn the_printer_list_is_read_from_lpstat() {
        let out =
            "printer EPSON-L3250-Series is idle.  enabled since Tue 21 Oct 2026 10:00:00 AM\n\
                   printer Office_HP disabled since Mon 20 Oct 2026 09:00:00 AM -\n\
                   \tPaused\n\
                   printer PDF is idle.  enabled since Tue 21 Oct 2026 10:00:00 AM\n\
                   system default destination: Office_HP\n";
        let p = parse_lpstat(out);
        let names: Vec<&str> = p.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Office_HP", "EPSON-L3250-Series", "PDF"]);
        assert!(p[0].default && !p[1].default && !p[2].default);
        // No printers, or no default: the list as given.
        assert!(parse_lpstat("lpstat: No destinations added.\n").is_empty());
        assert!(parse_lpstat("no system default destination\n").is_empty());
        let p =
            parse_lpstat("printer A is idle.\nprinter B is idle.\nno system default destination\n");
        assert_eq!(p.len(), 2);
        assert!(!p[0].default);
    }

    #[test]
    fn the_dialog_starts_on_the_remembered_printer_else_the_default() {
        let printers =
            parse_lpstat("printer A is idle.\nprinter B is idle.\nsystem default destination: B\n");
        assert_eq!(starting_printer(Some("A"), &printers).as_deref(), Some("A"));
        assert_eq!(
            starting_printer(Some("Gone"), &printers).as_deref(),
            Some("B")
        );
        assert_eq!(starting_printer(None, &printers).as_deref(), Some("B"));
        assert_eq!(starting_printer(Some("A"), &[]), None);
    }

    /// `lp` gets the printer, the copies with their collation, the title
    /// and the file; one copy needs no count.
    #[test]
    fn lp_is_told_the_printer_copies_and_title() {
        let pdf = Path::new("/tmp/tracedraw-print/Poster.pdf");
        assert_eq!(
            lp_args("EPSON-L3250-Series", pdf, 1, true, "Poster"),
            [
                "-d",
                "EPSON-L3250-Series",
                "-t",
                "Poster",
                "--",
                "/tmp/tracedraw-print/Poster.pdf"
            ]
        );
        assert_eq!(
            lp_args("A", pdf, 3, false, ""),
            [
                "-d",
                "A",
                "-n",
                "3",
                "-o",
                "collate=false",
                "--",
                "/tmp/tracedraw-print/Poster.pdf"
            ]
        );
        assert_eq!(
            lp_args("A", pdf, 2, true, " ")[4..6],
            ["-o", "collate=true"]
        );
    }
}
