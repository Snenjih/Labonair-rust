//! Lightweight language identification from a file path.
//!
//! T06-001 only needs the *identity* of the language (for the status bar and to
//! prepare the architecture); actual syntax highlighting via Tree-sitter is
//! T06-002, which will hang a grammar off each variant.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    Java,
    Php,
    Xml,
    Ruby,
    Swift,
    Kotlin,
}

impl Language {
    /// Every editor language exposed by the manual language-mode selector.
    pub const ALL: [Self; 22] = [
        Self::PlainText,
        Self::Rust,
        Self::Toml,
        Self::Json,
        Self::Yaml,
        Self::Markdown,
        Self::Html,
        Self::Css,
        Self::JavaScript,
        Self::TypeScript,
        Self::Python,
        Self::Go,
        Self::C,
        Self::Cpp,
        Self::Shell,
        Self::Sql,
        Self::Java,
        Self::Php,
        Self::Xml,
        Self::Ruby,
        Self::Swift,
        Self::Kotlin,
    ];

    /// Stable token used when a typed UI control crosses a string boundary.
    pub fn id(self) -> &'static str {
        match self {
            Self::PlainText => "plain-text",
            Self::Rust => "rust",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Css => "css",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Python => "python",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Shell => "shell",
            Self::Sql => "sql",
            Self::Java => "java",
            Self::Php => "php",
            Self::Xml => "xml",
            Self::Ruby => "ruby",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|language| language.id() == id)
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.label() == label)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match name.as_str() {
            "dockerfile" | "dockerfile.dev" | "containerfile" => return Language::Shell,
            "makefile" | "gnumakefile" => return Language::Shell,
            ".bashrc" | ".zshrc" | ".bash_profile" | ".profile" => return Language::Shell,
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
            "sh" | "bash" | "zsh" | "fish" | "ksh" => Language::Shell,
            "sql" => Language::Sql,
            "java" => Language::Java,
            "php" | "phtml" => Language::Php,
            "xml" | "svg" | "xsl" | "plist" => Language::Xml,
            "rb" | "rake" | "gemspec" => Language::Ruby,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
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
            Language::Java => "Java",
            Language::Php => "PHP",
            Language::Xml => "XML",
            Language::Ruby => "Ruby",
            Language::Swift => "Swift",
            Language::Kotlin => "Kotlin",
        }
    }

    /// Whether a bundled Tree-sitter grammar can highlight this language.
    pub fn has_grammar(&self) -> bool {
        matches!(
            self,
            Language::Rust
                | Language::Json
                | Language::Toml
                | Language::Yaml
                | Language::Python
                | Language::JavaScript
                | Language::TypeScript
                | Language::Go
                | Language::C
                | Language::Cpp
                | Language::Css
                | Language::Html
                | Language::Java
                | Language::Shell
        )
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
        assert_eq!(Language::from_path("Main.java"), Language::Java);
        assert_eq!(Language::from_path("a.rb"), Language::Ruby);
        assert_eq!(Language::from_path("q.sql"), Language::Sql);
        assert_eq!(Language::from_path("index.php"), Language::Php);
        assert_eq!(Language::from_path("m.kt"), Language::Kotlin);
        assert_eq!(Language::from_path("v.swift"), Language::Swift);
        assert_eq!(Language::from_path("data.yaml"), Language::Yaml);
    }

    #[test]
    fn grammar_availability_matches_bundled_set() {
        assert!(Language::Rust.has_grammar());
        assert!(Language::TypeScript.has_grammar());
        assert!(!Language::PlainText.has_grammar());
        assert!(!Language::Sql.has_grammar());
    }

    #[test]
    fn all_languages_have_unique_stable_ids_and_labels() {
        assert_eq!(Language::ALL.len(), 22);
        for (index, language) in Language::ALL.iter().enumerate() {
            assert_eq!(Language::from_id(language.id()), Some(*language));
            assert_eq!(Language::from_label(language.label()), Some(*language));
            assert!(!Language::ALL[..index]
                .iter()
                .any(|previous| previous.id() == language.id()));
        }
    }
}
