//! Lightweight language identification from a file path.
//!
//! T06-001 only needs the *identity* of the language (for the status bar and to
//! prepare the architecture); actual syntax highlighting via Tree-sitter is
//! T06-002, which will hang a grammar off each variant.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    PlainText,
    Rust,
    Toml,
    Json,
    Yaml,
    Markdown,
    Html,
    Css,
    JavaScript,
    TypeScript,
    Python,
    Go,
    C,
    Cpp,
    Shell,
    Sql,
}

impl Language {
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match name.as_str() {
            "dockerfile" => return Language::Shell,
            "makefile" => return Language::Shell,
            _ => {}
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "rs" => Language::Rust,
            "toml" => Language::Toml,
            "json" | "jsonc" => Language::Json,
            "yaml" | "yml" => Language::Yaml,
            "md" | "markdown" => Language::Markdown,
            "html" | "htm" => Language::Html,
            "css" | "scss" | "less" => Language::Css,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            "c" | "h" => Language::C,
            "cc" | "cpp" | "cxx" | "hpp" => Language::Cpp,
            "sh" | "bash" | "zsh" | "fish" => Language::Shell,
            "sql" => Language::Sql,
            _ => Language::PlainText,
        }
    }

    /// Human-readable label for the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            Language::PlainText => "Plain Text",
            Language::Rust => "Rust",
            Language::Toml => "TOML",
            Language::Json => "JSON",
            Language::Yaml => "YAML",
            Language::Markdown => "Markdown",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Python => "Python",
            Language::Go => "Go",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Shell => "Shell",
            Language::Sql => "SQL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_extensions() {
        assert_eq!(Language::from_path("src/main.rs"), Language::Rust);
        assert_eq!(Language::from_path("Cargo.toml"), Language::Toml);
        assert_eq!(Language::from_path("a/b/App.tsx"), Language::TypeScript);
        assert_eq!(Language::from_path("Dockerfile"), Language::Shell);
        assert_eq!(Language::from_path("notes"), Language::PlainText);
    }
}
