//! Dependency-free Emmet abbreviation expansion for editor previews.
//!
//! The markup parser handles element names, ids, classes, attributes, text,
//! child/sibling operators, groups, and bounded repetition. CSS supports common
//! property shorthands. Keeping this parser in the Editor owner lets the UI
//! offer a live preview without mutating the document until the user applies it.

use std::fmt;

use crate::Language;

const MAX_EMMET_NODES: usize = 256;
const MAX_EMMET_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmmetError {
    EmptyAbbreviation,
    InvalidAbbreviation(String),
    UnsupportedLanguage(Language),
    ExpansionTooLarge,
}

impl fmt::Display for EmmetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAbbreviation => f.write_str("Enter an Emmet abbreviation."),
            Self::InvalidAbbreviation(message) => f.write_str(message),
            Self::UnsupportedLanguage(language) => {
                write!(
                    f,
                    "Emmet expansion is unavailable for {}.",
                    language.label()
                )
            }
            Self::ExpansionTooLarge => {
                f.write_str("The abbreviation expands to too many elements.")
            }
        }
    }
}

impl std::error::Error for EmmetError {}

/// Expand one HTML-like or CSS Emmet abbreviation into source text.
pub fn expand_emmet_abbreviation(
    language: Language,
    abbreviation: &str,
) -> Result<String, EmmetError> {
    let abbreviation = abbreviation.trim();
    if abbreviation.is_empty() {
        return Err(EmmetError::EmptyAbbreviation);
    }
    if language == Language::Css {
        return expand_css(abbreviation);
    }
    if !matches!(
        language,
        Language::Html
            | Language::Xml
            | Language::JavaScript
            | Language::TypeScript
            | Language::Php
    ) {
        return Err(EmmetError::UnsupportedLanguage(language));
    }

    let mut parser = MarkupParser::new(abbreviation);
    let nodes = parser.parse_sequence(None, 0)?;
    if parser.cursor != abbreviation.len() || nodes.is_empty() {
        return Err(EmmetError::InvalidAbbreviation(
            "The abbreviation could not be parsed.".into(),
        ));
    }
    let mut output = String::new();
    for (index, node) in nodes.iter().enumerate() {
        render_node(node, 0, &mut output);
        if index + 1 < nodes.len() {
            output.push('\n');
        }
    }
    if output.ends_with('\n') {
        output.pop();
    }
    Ok(output)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Element {
    name: String,
    attributes: Vec<(String, Option<String>)>,
    text: Option<String>,
    children: Vec<Element>,
}

struct MarkupParser<'a> {
    input: &'a str,
    cursor: usize,
    nodes: usize,
}

impl<'a> MarkupParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            cursor: 0,
            nodes: 0,
        }
    }

    fn parse_sequence(
        &mut self,
        stop: Option<char>,
        depth: usize,
    ) -> Result<Vec<Element>, EmmetError> {
        if depth > MAX_EMMET_DEPTH {
            return Err(self.invalid("The abbreviation is nested too deeply."));
        }
        let mut output = Vec::new();
        loop {
            self.skip_spaces();
            if self.peek_char().is_none() || self.peek_char() == stop {
                break;
            }

            let mut term = self.parse_primary(depth + 1)?;
            let repeat = self.parse_repeat()?;
            if self.consume('>') {
                let children = self.parse_sequence(stop, depth + 1)?;
                if let Some(parent) = term.last_mut() {
                    parent.children.extend(children);
                } else {
                    return Err(self.invalid("A child operator needs a parent element."));
                }
            }
            if repeat > 1 {
                let original = term;
                self.add_nodes(
                    original
                        .iter()
                        .map(element_count)
                        .sum::<usize>()
                        .saturating_mul(repeat - 1),
                )?;
                term = (1..=repeat)
                    .flat_map(|iteration| {
                        original
                            .iter()
                            .cloned()
                            .map(move |node| replace_repeat_marker(node, iteration))
                    })
                    .collect();
            }
            output.extend(term);
            self.skip_spaces();
            if self.consume('+') {
                if self.peek_char().is_none() || self.peek_char() == stop {
                    return Err(self.invalid("A sibling operator needs another element."));
                }
            } else {
                break;
            }
            if output.len() > MAX_EMMET_NODES {
                return Err(EmmetError::ExpansionTooLarge);
            }
        }
        Ok(output)
    }

    fn parse_primary(&mut self, depth: usize) -> Result<Vec<Element>, EmmetError> {
        self.skip_spaces();
        if self.consume('(') {
            let nodes = self.parse_sequence(Some(')'), depth + 1)?;
            if !self.consume(')') {
                return Err(self.invalid("A group is missing its closing parenthesis."));
            }
            if nodes.is_empty() {
                return Err(self.invalid("An Emmet group cannot be empty."));
            }
            return Ok(nodes);
        }
        self.parse_element().map(|node| vec![node])
    }

    fn parse_element(&mut self) -> Result<Element, EmmetError> {
        let name_start = self.cursor;
        while self.peek_char().is_some_and(is_name_character) {
            self.advance_char();
        }
        let name = if self.cursor == name_start {
            "div".to_string()
        } else {
            self.input[name_start..self.cursor].to_string()
        };
        if !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) {
            return Err(self.invalid("An element name must start with a letter."));
        }

        let mut element = Element {
            name,
            ..Element::default()
        };
        while let Some(character) = self.peek_char() {
            match character {
                '.' => {
                    self.advance_char();
                    let class = self.parse_identifier()?;
                    if let Some((_, Some(classes))) = element
                        .attributes
                        .iter_mut()
                        .find(|(name, _)| name == "class")
                    {
                        classes.push(' ');
                        classes.push_str(&class);
                    } else {
                        element.attributes.push(("class".into(), Some(class)));
                    }
                }
                '#' => {
                    self.advance_char();
                    let id = self.parse_identifier()?;
                    element.attributes.push(("id".into(), Some(id)));
                }
                '[' => self.parse_attributes(&mut element)?,
                '{' => {
                    self.advance_char();
                    element.text = Some(self.parse_text()?);
                }
                _ => break,
            }
        }
        self.add_nodes(1)?;
        Ok(element)
    }

    fn parse_identifier(&mut self) -> Result<String, EmmetError> {
        let start = self.cursor;
        while self.peek_char().is_some_and(|character| {
            character.is_alphanumeric() || matches!(character, '_' | '-' | '$' | ':')
        }) {
            self.advance_char();
        }
        if start == self.cursor {
            return Err(self.invalid("A class or id marker needs a name."));
        }
        Ok(self.input[start..self.cursor].to_string())
    }

    fn parse_attributes(&mut self, element: &mut Element) -> Result<(), EmmetError> {
        self.advance_char();
        let start = self.cursor;
        let mut quote = None;
        while let Some(character) = self.peek_char() {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() && matches!(character, '\'' | '"') {
                quote = Some(character);
            } else if quote.is_none() && character == ']' {
                break;
            }
            self.advance_char();
        }
        if !self.consume(']') || quote.is_some() {
            return Err(self.invalid("An attribute list is not closed."));
        }
        let attributes = &self.input[start..self.cursor - 1];
        for attribute in split_attribute_tokens(attributes) {
            let Some((name, value)) = attribute.split_once('=') else {
                if attribute.is_empty() {
                    continue;
                }
                element.attributes.push((attribute.to_string(), None));
                continue;
            };
            if name.is_empty() {
                return Err(self.invalid("An attribute name cannot be empty."));
            }
            let value = value.trim_matches(['\'', '"']);
            element
                .attributes
                .push((name.to_string(), Some(value.to_string())));
        }
        Ok(())
    }

    fn parse_text(&mut self) -> Result<String, EmmetError> {
        let start = self.cursor;
        let mut depth = 1_usize;
        while let Some(character) = self.peek_char() {
            self.advance_char();
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(self.input[start..self.cursor - 1].to_string());
                    }
                }
                _ => {}
            }
        }
        Err(self.invalid("Text content is missing its closing brace."))
    }

    fn parse_repeat(&mut self) -> Result<usize, EmmetError> {
        if !self.consume('*') {
            return Ok(1);
        }
        let start = self.cursor;
        while self
            .peek_char()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance_char();
        }
        let repeat = self.input[start..self.cursor]
            .parse::<usize>()
            .map_err(|_| self.invalid("A repeat count must be a positive integer."))?;
        if repeat == 0 || repeat > MAX_EMMET_NODES {
            return Err(EmmetError::ExpansionTooLarge);
        }
        Ok(repeat)
    }

    fn add_nodes(&mut self, count: usize) -> Result<(), EmmetError> {
        self.nodes = self.nodes.saturating_add(count);
        if self.nodes > MAX_EMMET_NODES {
            Err(EmmetError::ExpansionTooLarge)
        } else {
            Ok(())
        }
    }

    fn skip_spaces(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.advance_char();
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.advance_char();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input.get(self.cursor..)?.chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(character) = self.peek_char() {
            self.cursor += character.len_utf8();
        }
    }

    fn invalid(&self, message: &str) -> EmmetError {
        EmmetError::InvalidAbbreviation(message.into())
    }
}

fn split_attribute_tokens(attributes: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut quote = None;
    for (index, character) in attributes.char_indices() {
        if quote == Some(character) {
            quote = None;
        } else if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if quote.is_none() && character.is_whitespace() {
            if let Some(start) = start.take() {
                tokens.push(&attributes[start..index]);
            }
        } else {
            start.get_or_insert(index);
        }
    }
    if let Some(start) = start {
        tokens.push(&attributes[start..]);
    }
    tokens
}

fn is_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_')
}

fn replace_repeat_marker(mut element: Element, iteration: usize) -> Element {
    let replacement = iteration.to_string();
    element.name = element.name.replace('$', &replacement);
    for (name, value) in &mut element.attributes {
        *name = name.replace('$', &replacement);
        if let Some(value) = value {
            *value = value.replace('$', &replacement);
        }
    }
    if let Some(text) = &mut element.text {
        *text = text.replace('$', &replacement);
    }
    element.children = element
        .children
        .into_iter()
        .map(|child| replace_repeat_marker(child, iteration))
        .collect();
    element
}

fn element_count(element: &Element) -> usize {
    1 + element.children.iter().map(element_count).sum::<usize>()
}

fn render_node(element: &Element, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);
    output.push_str(&indent);
    output.push('<');
    output.push_str(&element.name);
    for (name, value) in &element.attributes {
        output.push(' ');
        output.push_str(name);
        if let Some(value) = value {
            output.push_str("=\"");
            output.push_str(&escape_attribute(value));
            output.push('"');
        }
    }
    if is_void_element(&element.name) {
        output.push_str(" />");
        return;
    }
    output.push('>');
    if let Some(text) = &element.text {
        output.push_str(text);
    }
    if element.children.is_empty() {
        output.push_str("</");
        output.push_str(&element.name);
        output.push('>');
        return;
    }
    output.push('\n');
    for (index, child) in element.children.iter().enumerate() {
        render_node(child, depth + 1, output);
        if index + 1 < element.children.len() {
            output.push('\n');
        }
    }
    output.push('\n');
    output.push_str(&indent);
    output.push_str("</");
    output.push_str(&element.name);
    output.push('>');
}

fn escape_attribute(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn expand_css(abbreviation: &str) -> Result<String, EmmetError> {
    abbreviation
        .split('+')
        .map(expand_css_property)
        .collect::<Result<Vec<_>, _>>()
        .map(|properties| properties.join("\n"))
}

fn expand_css_property(abbreviation: &str) -> Result<String, EmmetError> {
    let (property_code, raw_value) = abbreviation
        .split_once(':')
        .map_or((abbreviation, None), |(property, value)| {
            (property, Some(value))
        });
    let (property, suffix_value) = if let Some(property) = css_property(property_code) {
        let value = &property_code[css_code_prefix_len(property_code).unwrap_or(0)..];
        (property, (!value.is_empty()).then_some(value))
    } else {
        return Err(EmmetError::InvalidAbbreviation(format!(
            "Unknown CSS Emmet property `{property_code}`."
        )));
    };
    let value = raw_value.or(suffix_value).unwrap_or("");
    let value = css_value(property, value);
    Ok(format!("{property}: {value};"))
}

fn css_code_prefix_len(value: &str) -> Option<usize> {
    code_prefix(value).map(str::len)
}

fn css_property(code: &str) -> Option<&'static str> {
    Some(match code_prefix(code)? {
        "bgc" => "background-color",
        "pos" => "position",
        "mt" => "margin-top",
        "mr" => "margin-right",
        "mb" => "margin-bottom",
        "ml" => "margin-left",
        "pt" => "padding-top",
        "pr" => "padding-right",
        "pb" => "padding-bottom",
        "pl" => "padding-left",
        "fs" => "font-size",
        "fw" => "font-weight",
        "lh" => "line-height",
        "ta" => "text-align",
        "m" => "margin",
        "p" => "padding",
        "w" => "width",
        "h" => "height",
        "d" => "display",
        "c" => "color",
        "t" => "top",
        _ => return None,
    })
}

fn code_prefix(value: &str) -> Option<&str> {
    [
        "bgc", "pos", "mt", "mr", "mb", "ml", "pt", "pr", "pb", "pl", "fs", "fw", "lh", "ta", "m",
        "p", "w", "h", "d", "c", "t",
    ]
    .into_iter()
    .filter(|code| value.starts_with(code))
    .max_by_key(|code| code.len())
}

fn css_value(property: &str, raw_value: &str) -> String {
    let mut value = raw_value.trim().to_string();
    if let Some(expanded) = value_mapping(property, &value) {
        value = expanded.to_string();
    }
    if value.is_empty() {
        return value;
    }
    if value.contains('-') && value.split('-').all(|part| part.parse::<f32>().is_ok()) {
        return value
            .split('-')
            .map(|part| format!("{part}px"))
            .collect::<Vec<_>>()
            .join(" ");
    }
    let numeric = value.parse::<f32>().is_ok();
    if numeric {
        if matches!(property, "font-weight" | "line-height") {
            return value;
        }
        value.push_str("px");
    }
    value
}

fn value_mapping(property: &str, value: &str) -> Option<&'static str> {
    Some(match (property, value) {
        ("display", "f") => "flex",
        ("display", "g") => "grid",
        ("display", "b") => "block",
        ("display", "i") => "inline",
        ("display", "ib") => "inline-block",
        ("display", "n") => "none",
        ("position", "r") => "relative",
        ("position", "a") => "absolute",
        ("position", "f") => "fixed",
        ("text-align", "c") => "center",
        ("text-align", "l") => "left",
        ("text-align", "r") => "right",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_expands_selectors_attributes_text_and_void_elements() {
        assert_eq!(
            expand_emmet_abbreviation(Language::Html, "a#docs.link[href=guide.html]{Read more}")
                .expect("valid abbreviation"),
            "<a id=\"docs\" class=\"link\" href=\"guide.html\">Read more</a>"
        );
        assert_eq!(
            expand_emmet_abbreviation(Language::Html, "img[src=logo.svg]")
                .expect("valid abbreviation"),
            "<img src=\"logo.svg\" />"
        );
        assert_eq!(
            expand_emmet_abbreviation(Language::Html, "a[title='a long label' aria-label=link]")
                .expect("quoted spaces are valid in attributes"),
            "<a title=\"a long label\" aria-label=\"link\"></a>"
        );
    }

    #[test]
    fn markup_expands_children_siblings_groups_and_repetition() {
        assert_eq!(
            expand_emmet_abbreviation(Language::Html, "ul>li.item$*3")
                .expect("valid abbreviation"),
            "<ul>\n  <li class=\"item1\"></li>\n  <li class=\"item2\"></li>\n  <li class=\"item3\"></li>\n</ul>"
        );
        assert_eq!(
            expand_emmet_abbreviation(Language::Html, "(header>h1)+main")
                .expect("valid abbreviation"),
            "<header>\n  <h1></h1>\n</header>\n<main></main>"
        );
    }

    #[test]
    fn css_expands_common_properties_and_values() {
        assert_eq!(
            expand_emmet_abbreviation(Language::Css, "m10+p20+d:f+ta:c")
                .expect("valid CSS abbreviation"),
            "margin: 10px;\npadding: 20px;\ndisplay: flex;\ntext-align: center;"
        );
    }

    #[test]
    fn invalid_and_unsupported_abbreviations_return_typed_errors() {
        assert_eq!(
            expand_emmet_abbreviation(Language::Html, " "),
            Err(EmmetError::EmptyAbbreviation)
        );
        assert!(matches!(
            expand_emmet_abbreviation(Language::Rust, "div"),
            Err(EmmetError::UnsupportedLanguage(Language::Rust))
        ));
        assert!(expand_emmet_abbreviation(Language::Html, "ul>li*").is_err());
        assert!(expand_emmet_abbreviation(Language::Css, "unknown:value").is_err());
        assert!(matches!(
            expand_emmet_abbreviation(Language::Html, "li*257"),
            Err(EmmetError::ExpansionTooLarge)
        ));
    }
}
