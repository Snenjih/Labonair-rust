//! CI guard for the Block A icon purge (T16-003).
//!
//! The reference app never uses emoji or geometric glyphs as icons — every
//! glyph is an SVG. This test greps the crate sources for the pseudo-icon
//! characters the port used to substitute (see `vergleichsbericht-subagent-3.md`
//! §2b) and fails if any survive in real code.
//!
//! Line comments (`//` / `//!`) and the trailing `#[cfg(test)]` module of each
//! file are excluded — decorative glyphs there (`→`, `·`, `▲`, status dots)
//! are explicitly allowed by the audit.

use std::fs;
use std::path::Path;

/// Characters that must never appear as a rendered icon.
///
/// Emoji are matched by range; the rest is the explicit list from the audit's
/// "geometric/technical glyphs used as icons" table. Deliberately *excluded*:
/// arrows (`→ ↔ ↑ ↓`), the middle dot `·`, ellipsis `…`, em dash `—`,
/// disclosure carets `▸ ▾`, status dots `○ ◐ ● ◌`, check/cross marks used in
/// log strings — all sanctioned by the audit as decorative.
const FORBIDDEN: &[char] = &[
    '\u{2699}', // ⚙ gear
    '\u{2702}', // ✂ scissors
    '\u{2728}', // ✨ sparkles
    '\u{270E}', // ✎ pencil
    '\u{270F}', // ✏ pencil
    '\u{26D3}', // ⛓ chains
    '\u{26A0}', // ⚠ warning
    '\u{2611}', // ☑ checked box
    '\u{2610}', // ☐ empty box
    '\u{2302}', // ⌂ house
    '\u{25C8}', // ◈
    '\u{2726}', // ✦
    '\u{21C5}', // ⇅
    '\u{2387}', // ⎇
    '\u{29C9}', // ⧉
    '\u{21BB}', // ↻ refresh
    '\u{2933}', // ⤳
    '\u{FF0B}', // ＋ fullwidth plus
];

fn is_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF)
}

#[test]
fn no_pictograph_icons_in_sources() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    visit(&src, &mut offenders);
    assert!(
        offenders.is_empty(),
        "pseudo-icon glyphs found in crate sources (use `crate::components::IconName`):\n{}",
        offenders.join("\n")
    );
}

fn visit(dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            scan_file(&path, out);
        }
    }
}

fn scan_file(path: &Path, out: &mut Vec<String>) {
    let text = fs::read_to_string(path).unwrap();
    for (n, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("#[cfg(test)]") {
            // Colocated unit tests live at the end of the file — stop here.
            return;
        }
        for c in line.chars() {
            if is_emoji(c) || FORBIDDEN.contains(&c) {
                out.push(format!(
                    "{}:{}: U+{:04X} {c}",
                    path.file_name().unwrap().to_string_lossy(),
                    n + 1,
                    c as u32
                ));
            }
        }
    }
}
