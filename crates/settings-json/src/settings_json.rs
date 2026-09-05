//! Surgical single-node edits to a `settings.json` text (T19-005).
//!
//! Port of `zed-refrence/zed/crates/settings_json/src/settings_json.rs`,
//! trimmed to what this task needs: no `editing` cargo feature gate (always
//! compiled — this crate exists *only* for editing), and
//! `parse_json_with_comments` dropped (labonair already has a JSONC-tolerant
//! parse path in `labonair-settings-content`/`jsonc-parser`, used by
//! `labonair_settings::SettingsStore` to validate + parse the file; this
//! crate's job is strictly "given old/new `SettingsContent`-shaped JSON
//! values, produce the minimal text edit").
//!
//! The core idea (`update_value_in_json_text`): walk the old and new value
//! trees together. Where a sub-object is unchanged, recurse without
//! touching the text. Where a leaf differs, replace only that leaf's byte
//! span (found via a real JSON syntax tree, so `//`/`/* */` comments and
//! trailing commas around it are never disturbed). Where a key is missing
//! entirely, insert it at the right place with matching indentation.

use serde::Serialize;
use serde_json::Value;
use std::ops::Range;
use std::sync::LazyLock;
use tree_sitter::{Query, StreamingIterator as _};

/// `Range::contains` extended to also accept `other == self` at the
/// endpoints (`zed_util::RangeExt::contains_inclusive`, inlined here so this
/// crate has no dependency on Zed's `util` crate).
fn contains_inclusive(container: &Range<usize>, other: &Range<usize>) -> bool {
    container.start <= other.start && other.end <= container.end
}

pub fn update_value_in_json_text<'a>(
    text: &mut String,
    key_path: &mut Vec<&'a str>,
    tab_size: usize,
    old_value: &'a Value,
    new_value: &'a Value,
    edits: &mut Vec<(Range<usize>, String)>,
) {
    // If the old and new values are both objects, then compare them key by key,
    // preserving the comments and formatting of the unchanged parts. Otherwise,
    // replace the old value with the new value.
    if let (Value::Object(old_object), Value::Object(new_object)) = (old_value, new_value) {
        for (key, old_sub_value) in old_object.iter() {
            key_path.push(key);
            if let Some(new_sub_value) = new_object.get(key) {
                // Key exists in both old and new, recursively update
                update_value_in_json_text(
                    text,
                    key_path,
                    tab_size,
                    old_sub_value,
                    new_sub_value,
                    edits,
                );
            } else {
                // Key was removed from new object, remove the entire key-value pair
                let (range, replacement) =
                    replace_value_in_json_text(text, key_path, 0, None, None);
                text.replace_range(range.clone(), &replacement);
                edits.push((range, replacement));
            }
            key_path.pop();
        }
        for (key, new_sub_value) in new_object.iter() {
            key_path.push(key);
            if !old_object.contains_key(key) {
                update_value_in_json_text(
                    text,
                    key_path,
                    tab_size,
                    &Value::Null,
                    new_sub_value,
                    edits,
                );
            }
            key_path.pop();
        }
    } else if old_value != new_value {
        let mut new_value = new_value.clone();
        if let Some(new_object) = new_value.as_object_mut() {
            new_object.retain(|_, v| !v.is_null());
        }
        let (range, replacement) =
            replace_value_in_json_text(text, key_path, tab_size, Some(&new_value), None);
        text.replace_range(range.clone(), &replacement);
        edits.push((range, replacement));
    }
}

/// * `replace_key` - When an exact key match according to `key_path` is found, replace the key with `replace_key` if `Some`.
pub fn replace_value_in_json_text<T: AsRef<str>>(
    text: &str,
    key_path: &[T],
    tab_size: usize,
    new_value: Option<&Value>,
    replace_key: Option<&str>,
) -> (Range<usize>, String) {
    static PAIR_QUERY: LazyLock<Query> = LazyLock::new(|| {
        Query::new(
            &tree_sitter_json::LANGUAGE.into(),
            "(pair key: (string) @key value: (_) @value)",
        )
        .expect("Failed to create PAIR_QUERY")
    });

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .expect("failed to load tree-sitter-json grammar");
    let syntax_tree = parser
        .parse(text, None)
        .expect("tree-sitter-json parser has no timeout/cancellation configured");

    let mut cursor = tree_sitter::QueryCursor::new();

    let mut depth = 0;
    let mut last_value_range = 0..0;
    let mut first_key_start = None;
    let mut existing_value_range = 0..text.len();

    let mut matches = cursor.matches(&PAIR_QUERY, syntax_tree.root_node(), text.as_bytes());
    while let Some(mat) = matches.next() {
        if mat.captures.len() != 2 {
            continue;
        }

        let key_range = mat.captures[0].node.byte_range();
        let value_range = mat.captures[1].node.byte_range();

        // Don't enter sub objects until we find an exact
        // match for the current keypath
        if contains_inclusive(&last_value_range, &value_range) {
            continue;
        }

        last_value_range = value_range.clone();

        if key_range.start > existing_value_range.end {
            break;
        }

        first_key_start.get_or_insert(key_range.start);

        let found_key = text
            .get(key_range.clone())
            .zip(key_path.get(depth))
            .and_then(|(key_text, key_path_value)| {
                serde_json::to_string(key_path_value.as_ref())
                    .ok()
                    .map(|key_path| depth < key_path.len() && key_text == key_path)
            })
            .unwrap_or(false);

        if found_key {
            existing_value_range = value_range;
            // Reset last value range when increasing in depth
            last_value_range = existing_value_range.start..existing_value_range.start;
            depth += 1;

            if depth == key_path.len() {
                break;
            }

            if let Some(array_replacement) = handle_possible_array_value(
                &mat.captures[0].node,
                &mat.captures[1].node,
                text,
                &key_path[depth..],
                new_value,
                replace_key,
                tab_size,
            ) {
                return array_replacement;
            }

            first_key_start = None;
        }
    }

    // We found the exact key we want
    if depth == key_path.len() {
        if let Some(new_value) = new_value {
            let new_val = to_pretty_json(new_value, tab_size, tab_size * depth);
            if let Some(replace_key) = replace_key.and_then(|str| serde_json::to_string(str).ok()) {
                let new_key = format!("{}: ", replace_key);
                if let Some(key_start) = text[..existing_value_range.start].rfind('"') {
                    if let Some(prev_key_start) = text[..key_start].rfind('"') {
                        existing_value_range.start = prev_key_start;
                    } else {
                        existing_value_range.start = key_start;
                    }
                }
                (existing_value_range, new_key + &new_val)
            } else {
                (existing_value_range, new_val)
            }
        } else {
            let mut removal_start = first_key_start.unwrap_or(existing_value_range.start);
            let mut removal_end = existing_value_range.end;

            // Find the actual key position by looking for the key in the pair
            // We need to extend the range to include the key, not just the value
            if let Some(key_start) = text[..existing_value_range.start].rfind('"') {
                if let Some(prev_key_start) = text[..key_start].rfind('"') {
                    removal_start = prev_key_start;
                } else {
                    removal_start = key_start;
                }
            }

            let mut removed_comma = false;
            // Look backward for a preceding comma first
            let preceding_text = text.get(0..removal_start).unwrap_or("");
            if let Some(comma_pos) = preceding_text.rfind(',') {
                // Check if there are only whitespace characters between the comma and our key
                let between_comma_and_key = text.get(comma_pos + 1..removal_start).unwrap_or("");
                if between_comma_and_key.trim().is_empty() {
                    removal_start = comma_pos;
                    removed_comma = true;
                }
            }
            if !removed_comma {
                if let Some(remaining_text) = text.get(existing_value_range.end..) {
                    let mut chars = remaining_text.char_indices();
                    while let Some((offset, ch)) = chars.next() {
                        if ch == ',' {
                            removal_end = existing_value_range.end + offset + 1;
                            // Also consume whitespace after the comma
                            for (_, next_ch) in chars.by_ref() {
                                if next_ch.is_whitespace() {
                                    removal_end += next_ch.len_utf8();
                                } else {
                                    break;
                                }
                            }
                            break;
                        } else if !ch.is_whitespace() {
                            break;
                        }
                    }
                }
            }
            (removal_start..removal_end, String::new())
        }
    } else if let Some(first_key_start) = first_key_start {
        // We have key paths, construct the sub objects
        let new_key = key_path[depth].as_ref();
        // We don't have the key, construct the nested objects
        let new_value = construct_json_value(&key_path[(depth + 1)..], new_value);

        let mut row = 0;
        let mut column = 0;
        for (ix, char) in text.char_indices() {
            if ix == first_key_start {
                break;
            }
            if char == '\n' {
                row += 1;
                column = 0;
            } else {
                column += char.len_utf8();
            }
        }

        if row > 0 {
            // depth is 0 based, but division needs to be 1 based.
            let new_val = to_pretty_json(&new_value, column / (depth + 1), column);
            let space = ' ';
            let content = format!("\"{new_key}\": {new_val},\n{space:width$}", width = column);
            (first_key_start..first_key_start, content)
        } else {
            let new_val = serde_json::to_string(&new_value).unwrap();
            let mut content = format!(r#""{new_key}": {new_val},"#);
            content.push(' ');
            (first_key_start..first_key_start, content)
        }
    } else {
        // We don't have the key, construct the nested objects
        let new_value = construct_json_value(&key_path[depth..], new_value);
        let indent_prefix_len = tab_size * depth;
        let mut new_val = to_pretty_json(&new_value, tab_size, indent_prefix_len);
        if depth == 0 {
            new_val.push('\n');
        }
        // best effort to keep comments with best effort indentation
        let mut replace_text = &text[existing_value_range.clone()];
        while let Some(comment_start) = replace_text.rfind("//") {
            if let Some(comment_end) = replace_text[comment_start..].find('\n') {
                let mut comment_with_indent_start = replace_text[..comment_start]
                    .rfind('\n')
                    .unwrap_or(comment_start);
                if !replace_text[comment_with_indent_start..comment_start]
                    .trim()
                    .is_empty()
                {
                    comment_with_indent_start = comment_start;
                }
                new_val.insert_str(
                    1,
                    &replace_text[comment_with_indent_start..comment_start + comment_end],
                );
            }
            replace_text = &replace_text[..comment_start];
        }

        (existing_value_range, new_val)
    }
}

fn construct_json_value(
    key_path: &[impl AsRef<str>],
    new_value: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut new_value =
        serde_json::to_value(new_value.unwrap_or(&serde_json::Value::Null)).unwrap();
    for key in key_path.iter().rev() {
        if parse_index_key(key.as_ref()).is_some() {
            new_value = serde_json::json!([new_value]);
        } else {
            new_value = serde_json::json!({ key.as_ref().to_string(): new_value });
        }
    }
    new_value
}

fn parse_index_key(index_key: &str) -> Option<usize> {
    index_key.strip_prefix('#')?.parse().ok()
}

#[allow(clippy::too_many_arguments)]
fn handle_possible_array_value(
    key_node: &tree_sitter::Node,
    value_node: &tree_sitter::Node,
    text: &str,
    remaining_key_path: &[impl AsRef<str>],
    new_value: Option<&Value>,
    replace_key: Option<&str>,
    tab_size: usize,
) -> Option<(Range<usize>, String)> {
    if remaining_key_path.is_empty() {
        return None;
    }
    let key_path = remaining_key_path;
    let index = parse_index_key(key_path[0].as_ref())?;

    let value_is_array = value_node.kind() == TS_ARRAY_KIND;

    let array_str = if value_is_array {
        &text[value_node.byte_range()]
    } else {
        ""
    };

    let (mut replace_range, mut replace_value) = replace_top_level_array_value_in_json_text(
        array_str,
        &key_path[1..],
        new_value,
        replace_key,
        index,
        tab_size,
    );

    if value_is_array {
        replace_range.start += value_node.start_byte();
        replace_range.end += value_node.start_byte();
    } else {
        // replace the full value if it wasn't an array
        replace_range = value_node.byte_range();
    }
    let non_whitespace_char_count = replace_value.len()
        - replace_value
            .chars()
            .filter(char::is_ascii_whitespace)
            .count();
    let needs_indent = replace_value.ends_with('\n')
        || (replace_value
            .chars()
            .zip(replace_value.chars().skip(1))
            .any(|(c, next_c)| c == '\n' && !next_c.is_ascii_whitespace()));
    let contains_comment = (replace_value.contains("//") && replace_value.contains('\n'))
        || (replace_value.contains("/*") && replace_value.contains("*/"));
    if needs_indent {
        let indent_width = key_node.start_position().column;
        let increased_indent = format!("\n{space:width$}", space = ' ', width = indent_width);
        replace_value = replace_value.replace('\n', &increased_indent);
    } else if non_whitespace_char_count < 32 && !contains_comment {
        // remove indentation
        while let Some(idx) = replace_value.find("\n ") {
            replace_value.remove(idx);
        }
        while let Some(idx) = replace_value.find("  ") {
            replace_value.remove(idx);
        }
    }
    Some((replace_range, replace_value))
}

const TS_DOCUMENT_KIND: &str = "document";
const TS_ARRAY_KIND: &str = "array";
const TS_COMMENT_KIND: &str = "comment";

pub fn replace_top_level_array_value_in_json_text(
    text: &str,
    key_path: &[impl AsRef<str>],
    new_value: Option<&Value>,
    replace_key: Option<&str>,
    array_index: usize,
    tab_size: usize,
) -> (Range<usize>, String) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .expect("failed to load tree-sitter-json grammar");

    let syntax_tree = parser
        .parse(text, None)
        .expect("tree-sitter-json parser has no timeout/cancellation configured");

    let mut cursor = syntax_tree.walk();

    if cursor.node().kind() == TS_DOCUMENT_KIND {
        cursor.goto_first_child();
    }

    while cursor.node().kind() != TS_ARRAY_KIND {
        if !cursor.goto_next_sibling() {
            let json_value = construct_json_value(key_path, new_value);
            let json_value = serde_json::json!([json_value]);
            return (0..text.len(), to_pretty_json(&json_value, tab_size, 0));
        }
    }

    // false if no children
    cursor.goto_first_child();
    debug_assert_eq!(cursor.node().kind(), "[");

    let mut index = 0;

    while index <= array_index {
        let node = cursor.node();
        if !matches!(node.kind(), "[" | "]" | TS_COMMENT_KIND | ",")
            && !node.is_extra()
            && !node.is_missing()
        {
            if index == array_index {
                break;
            }
            index += 1;
        }
        if !cursor.goto_next_sibling() {
            if let Some(new_value) = new_value {
                return append_top_level_array_value_in_json_text(text, new_value, tab_size);
            } else {
                return (0..0, String::new());
            }
        }
    }

    let range = cursor.node().range();
    let indent_width = range.start_point.column;
    let offset = range.start_byte;
    let text_range = range.start_byte..range.end_byte;
    let value_str = &text[text_range.clone()];
    let needs_indent = range.start_point.row > 0;

    if new_value.is_none() && key_path.is_empty() {
        let mut remove_range = text_range;
        if index == 0 {
            while cursor.goto_next_sibling()
                && (cursor.node().is_extra() || cursor.node().is_missing())
            {}
            if cursor.node().kind() == "," {
                remove_range.end = cursor.node().range().end_byte;
            }
            if let Some(next_newline) = &text[remove_range.end + 1..].find('\n') {
                if text[remove_range.end + 1..remove_range.end + next_newline]
                    .chars()
                    .all(|c| c.is_ascii_whitespace())
                {
                    remove_range.end += next_newline;
                }
            }
        } else {
            while cursor.goto_previous_sibling()
                && (cursor.node().is_extra() || cursor.node().is_missing())
            {}
            if cursor.node().kind() == "," {
                remove_range.start = cursor.node().range().start_byte;
            }
        }
        (remove_range, String::new())
    } else {
        if let Some(array_replacement) = handle_possible_array_value(
            &cursor.node(),
            &cursor.node(),
            text,
            key_path,
            new_value,
            replace_key,
            tab_size,
        ) {
            return array_replacement;
        }
        let (mut replace_range, mut replace_value) =
            replace_value_in_json_text(value_str, key_path, tab_size, new_value, replace_key);

        replace_range.start += offset;
        replace_range.end += offset;

        if needs_indent {
            let increased_indent = format!("\n{space:width$}", space = ' ', width = indent_width);
            replace_value = replace_value.replace('\n', &increased_indent);
        } else {
            while let Some(idx) = replace_value.find("\n ") {
                replace_value.remove(idx + 1);
            }
            while let Some(idx) = replace_value.find('\n') {
                replace_value.replace_range(idx..idx + 1, " ");
            }
        }

        (replace_range, replace_value)
    }
}

pub fn append_top_level_array_value_in_json_text(
    text: &str,
    new_value: &Value,
    tab_size: usize,
) -> (Range<usize>, String) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .expect("failed to load tree-sitter-json grammar");
    let syntax_tree = parser
        .parse(text, None)
        .expect("tree-sitter-json parser has no timeout/cancellation configured");

    let mut cursor = syntax_tree.walk();

    if cursor.node().kind() == TS_DOCUMENT_KIND {
        cursor.goto_first_child();
    }

    while cursor.node().kind() != TS_ARRAY_KIND {
        if !cursor.goto_next_sibling() {
            let json_value = serde_json::json!([new_value]);
            return (0..text.len(), to_pretty_json(&json_value, tab_size, 0));
        }
    }

    let went_to_last_child = cursor.goto_last_child();
    debug_assert!(
        went_to_last_child && cursor.node().kind() == "]",
        "Malformed JSON syntax tree, expected `]` at end of array"
    );
    let close_bracket_start = cursor.node().start_byte();
    while cursor.goto_previous_sibling()
        && (cursor.node().is_extra() || cursor.node().is_missing())
        && !cursor.node().is_error()
    {}

    let mut comma_range = None;
    let mut prev_item_range = None;

    if cursor.node().kind() == "," || is_error_of_kind(&mut cursor, ",") {
        comma_range = Some(cursor.node().byte_range());
        while cursor.goto_previous_sibling()
            && (cursor.node().is_extra() || cursor.node().is_missing())
        {}

        debug_assert_ne!(cursor.node().kind(), "[");
        prev_item_range = Some(cursor.node().range());
    } else {
        while (cursor.node().is_extra() || cursor.node().is_missing())
            && cursor.goto_previous_sibling()
        {}
        if cursor.node().kind() != "[" {
            prev_item_range = Some(cursor.node().range());
        }
    }

    let (mut replace_range, mut replace_value) =
        replace_value_in_json_text::<&str>("", &[], tab_size, Some(new_value), None);

    replace_range.start = close_bracket_start;
    replace_range.end = close_bracket_start;

    let space = ' ';
    if let Some(prev_item_range) = prev_item_range {
        let needs_newline = prev_item_range.start_point.row > 0;
        let indent_width = text[..prev_item_range.start_byte].rfind('\n').map_or(
            prev_item_range.start_point.column,
            |idx| {
                prev_item_range.start_point.column
                    - text[idx + 1..prev_item_range.start_byte].trim_start().len()
            },
        );

        let prev_item_end = comma_range
            .as_ref()
            .map_or(prev_item_range.end_byte, |range| range.end);
        if text[prev_item_end..replace_range.start].trim().is_empty() {
            replace_range.start = prev_item_end;
        }

        if needs_newline {
            let increased_indent = format!("\n{space:width$}", width = indent_width);
            replace_value = replace_value.replace('\n', &increased_indent);
            replace_value.push('\n');
            replace_value.insert_str(0, &format!("\n{space:width$}", width = indent_width));
        } else {
            while let Some(idx) = replace_value.find("\n ") {
                replace_value.remove(idx + 1);
            }
            while let Some(idx) = replace_value.find('\n') {
                replace_value.replace_range(idx..idx + 1, " ");
            }
            replace_value.insert(0, ' ');
        }

        if comma_range.is_none() {
            replace_value.insert(0, ',');
        }
    } else if replace_value.contains('\n') || text.contains('\n') {
        if let Some(prev_newline) = text[..replace_range.start].rfind('\n') {
            if text[prev_newline..replace_range.start].trim().is_empty() {
                replace_range.start = prev_newline;
            }
        }
        let indent = format!("\n{space:width$}", width = tab_size);
        replace_value = replace_value.replace('\n', &indent);
        replace_value.insert_str(0, &indent);
        replace_value.push('\n');
    }
    return (replace_range, replace_value);

    fn is_error_of_kind(cursor: &mut tree_sitter::TreeCursor<'_>, kind: &str) -> bool {
        if cursor.node().kind() != "ERROR" {
            return false;
        }

        let descendant_index = cursor.descendant_index();
        let res = cursor.goto_first_child() && cursor.node().kind() == kind;
        cursor.goto_descendant(descendant_index);
        res
    }
}

/// Infers the indentation size used in JSON text by analyzing the tree structure.
/// Returns the detected indent size, or a default of 2 if no indentation is found.
pub fn infer_json_indent_size(text: &str) -> usize {
    const MAX_INDENT_SIZE: usize = 64;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .expect("failed to load tree-sitter-json grammar");

    let Some(syntax_tree) = parser.parse(text, None) else {
        return 4;
    };

    let mut cursor = syntax_tree.walk();
    let mut indent_counts = [0u32; MAX_INDENT_SIZE];

    // Traverse the tree to find indentation patterns
    fn visit_node(
        cursor: &mut tree_sitter::TreeCursor,
        indent_counts: &mut [u32; MAX_INDENT_SIZE],
        depth: usize,
    ) {
        if depth >= 3 {
            return;
        }
        let node = cursor.node();
        let node_kind = node.kind();

        // For objects and arrays, check the indentation of their first content child
        if matches!(node_kind, "object" | "array") {
            let container_column = node.start_position().column;
            let container_row = node.start_position().row;

            if cursor.goto_first_child() {
                // Skip the opening bracket
                loop {
                    let child = cursor.node();
                    let child_kind = child.kind();

                    // Look for the first actual content (pair for objects, value for arrays)
                    if (node_kind == "object" && child_kind == "pair")
                        || (node_kind == "array"
                            && !matches!(child_kind, "[" | "]" | "," | "comment"))
                    {
                        let child_column = child.start_position().column;
                        let child_row = child.start_position().row;

                        // Only count if the child is on a different line
                        if child_row > container_row && child_column > container_column {
                            let indent = child_column - container_column;
                            if indent > 0 && indent < MAX_INDENT_SIZE {
                                indent_counts[indent] += 1;
                            }
                        }
                        break;
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        // Recurse to children
        if cursor.goto_first_child() {
            loop {
                visit_node(cursor, indent_counts, depth + 1);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    visit_node(&mut cursor, &mut indent_counts, 0);

    // Find the indent size with the highest count
    let mut max_count = 0;
    let mut max_indent = 4;

    for (indent, &count) in indent_counts.iter().enumerate() {
        if count > max_count {
            max_count = count;
            max_indent = indent;
        }
    }

    if max_count == 0 {
        2
    } else {
        max_indent
    }
}

pub fn to_pretty_json(
    value: &impl Serialize,
    indent_size: usize,
    indent_prefix_len: usize,
) -> String {
    let mut output = Vec::new();
    let indent = " ".repeat(indent_size);
    let mut ser = serde_json::Serializer::with_formatter(
        &mut output,
        serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes()),
    );

    value.serialize(&mut ser).unwrap();
    let text = String::from_utf8(output).unwrap();

    let mut adjusted_text = String::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            adjusted_text.extend(std::iter::repeat_n(' ', indent_prefix_len));
        }
        adjusted_text.push_str(line);
        adjusted_text.push('\n');
    }
    adjusted_text.pop();
    adjusted_text
}

/// Read-only lookup: the byte range of the value at `key_path` in `text`, if
/// present (T19-006 — settings.json schema-validation errors want a line
/// number next to their `json_path`, and this crate already parses the file
/// into a real `tree-sitter-json` syntax tree for the surgical-edit path
/// above). Returns `None` if any segment of `key_path` isn't found (e.g. the
/// key is missing, or a non-object value is indexed into) — callers fall
/// back to reporting the `json_path` alone, no line, which is an accepted
/// degraded mode per this task's Warnungen.
pub fn find_value_range(text: &str, key_path: &[&str]) -> Option<Range<usize>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(text, None)?;
    let mut node = tree.root_node();
    if node.kind() == "document" {
        node = node.named_child(0)?;
    }

    for key in key_path {
        if node.kind() != "object" {
            return None;
        }
        let mut cursor = node.walk();
        let mut found = None;
        for child in node.named_children(&mut cursor) {
            if child.kind() != "pair" {
                continue;
            }
            let key_node = child.child_by_field_name("key")?;
            let key_text = text.get(key_node.byte_range())?.trim_matches('"');
            if key_text == *key {
                found = child.child_by_field_name("value");
                break;
            }
        }
        node = found?;
    }

    Some(node.byte_range())
}

/// The dotted key path leading to the JSON value at `offset` (a byte offset
/// into `text`), if `offset` falls inside some nested object's pair — used
/// by the settings-editor's schema-hover helper (T19-006 Anweisung #5):
/// hovering the mouse over a key/value in `config.json` needs to
/// know which `SettingsContent` field is under the cursor. Returns `None`
/// if `offset` isn't inside any pair (e.g. it's on punctuation/whitespace
/// between top-level entries, or the text doesn't parse).
pub fn json_path_at_offset(text: &str, offset: usize) -> Option<Vec<String>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(text, None)?;
    let mut node = tree.root_node();
    if node.kind() == "document" {
        node = node.named_child(0)?;
    }

    let mut path = Vec::new();
    loop {
        if node.kind() != "object" || !node.byte_range().contains(&offset) {
            break;
        }
        let mut cursor = node.walk();
        let mut matched = None;
        for child in node.named_children(&mut cursor) {
            if child.kind() != "pair" || !child.byte_range().contains(&offset) {
                continue;
            }
            let key_node = child.child_by_field_name("key")?;
            let key_text = text
                .get(key_node.byte_range())?
                .trim_matches('"')
                .to_string();
            let value_node = child.child_by_field_name("value")?;
            matched = Some((key_text, value_node));
            break;
        }
        match matched {
            Some((key, value_node)) => {
                path.push(key);
                node = value_node;
            }
            None => break,
        }
    }

    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

/// 1-based `(line, column)` for a byte offset into `text` (columns counted in
/// UTF-8 bytes on the line, which is what most editors show for a JSON
/// document since it's ASCII-heavy). Used to turn [`find_value_range`]'s
/// byte offset into the human-readable position a validation-error banner
/// wants.
pub fn line_col_at(text: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for ch in text[..byte_offset.min(text.len())].chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += ch.len_utf8();
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use unindent::Unindent;

    #[test]
    fn object_replace() {
        #[track_caller]
        fn check_object_replace(
            input: String,
            key_path: &[&str],
            value: Option<Value>,
            expected: String,
        ) {
            let result = replace_value_in_json_text(&input, key_path, 4, value.as_ref(), None);
            let mut result_str = input;
            result_str.replace_range(result.0, &result.1);
            pretty_assertions::assert_eq!(expected, result_str);
        }
        check_object_replace(
            r#"{
                "a": 1,
                "b": 2
            }"#
            .unindent(),
            &["b"],
            Some(json!(3)),
            r#"{
                "a": 1,
                "b": 3
            }"#
            .unindent(),
        );
        check_object_replace(
            r#"{
                "a": 1,
                "b": 2
            }"#
            .unindent(),
            &["c"],
            Some(json!(3)),
            r#"{
                "c": 3,
                "a": 1,
                "b": 2
            }"#
            .unindent(),
        );
        check_object_replace(
            r#"{
                "name": "old_name",
                "id": 123
            }"#
            .unindent(),
            &["name"],
            Some(json!("new_name")),
            r#"{
                "name": "new_name",
                "id": 123
            }"#
            .unindent(),
        );
        check_object_replace(
            r#"{
                // This is a comment
                "a": 1,
                "b": 2 // Another comment
            }"#
            .unindent(),
            &["b"],
            Some(json!({"foo": "bar"})),
            r#"{
                // This is a comment
                "a": 1,
                "b": {
                    "foo": "bar"
                } // Another comment
            }"#
            .unindent(),
        );
        check_object_replace(
            r#"{}"#.to_string(),
            &["new_key"],
            Some(json!("value")),
            r#"{
                "new_key": "value"
            }
            "#
            .unindent(),
        );
        check_object_replace(
            r#"{
                "level1": {
                    "level2": {
                        "level3": {
                            "target": "old"
                        }
                    }
                }
            }"#
            .unindent(),
            &["level1", "level2", "level3", "target"],
            Some(json!("new")),
            r#"{
                "level1": {
                    "level2": {
                        "level3": {
                            "target": "new"
                        }
                    }
                }
            }"#
            .unindent(),
        );
        check_object_replace(
            r#"{
                "parent": {}
            }"#
            .unindent(),
            &["parent", "child"],
            Some(json!("value")),
            r#"{
                "parent": {
                    "child": "value"
                }
            }"#
            .unindent(),
        );
    }

    #[test]
    fn test_infer_json_indent_size() {
        let json_2_spaces = r#"{
  "key1": "value1",
  "nested": {
    "key2": "value2"
  }
}"#;
        assert_eq!(infer_json_indent_size(json_2_spaces), 2);

        let json_4_spaces = r#"{
    "key1": "value1",
    "nested": {
        "key2": "value2"
    }
}"#;
        assert_eq!(infer_json_indent_size(json_4_spaces), 4);

        let json_empty = r#"{}"#;
        assert_eq!(infer_json_indent_size(json_empty), 2);
    }

    #[test]
    fn update_value_in_json_text_updates_only_changed_leaves() {
        let mut text = r#"{
            // top-level comment
            "terminal": {
                "terminalFontSize": 14
            },
            "preferences": {
                "theme": "dark"
            }
        }"#
        .unindent();
        let old = json!({"terminal": {"terminalFontSize": 14}});
        let new = json!({"terminal": {"terminalFontSize": 20}});
        let mut edits = Vec::new();
        update_value_in_json_text(&mut text, &mut Vec::new(), 4, &old, &new, &mut edits);
        assert_eq!(edits.len(), 1);
        assert!(text.contains("\"terminalFontSize\": 20"));
        assert!(text.contains("\"theme\": \"dark\""));
        assert!(text.contains("// top-level comment"));
    }

    #[test]
    fn update_value_in_json_text_inserts_missing_key() {
        let mut text = "{}".to_string();
        let old = json!({});
        let new = json!({"terminal": {"terminalFontSize": 20}});
        let mut edits = Vec::new();
        update_value_in_json_text(&mut text, &mut Vec::new(), 4, &old, &new, &mut edits);
        assert!(!edits.is_empty());
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["terminal"]["terminalFontSize"], 20);
    }

    #[test]
    fn find_value_range_locates_nested_key() {
        let text = "{\n  \"terminal\": {\n    \"terminalFontSize\": \"gross\"\n  }\n}";
        let range = find_value_range(text, &["terminal", "terminalFontSize"]).unwrap();
        assert_eq!(&text[range], "\"gross\"");
    }

    #[test]
    fn find_value_range_missing_key_is_none() {
        let text = "{\"terminal\": {}}";
        assert!(find_value_range(text, &["terminal", "terminalFontSize"]).is_none());
    }

    #[test]
    fn json_path_at_offset_finds_nested_key() {
        let text = "{\n  \"terminal\": {\n    \"terminalFontSize\": 14\n  }\n}";
        let offset = text.find("14").unwrap();
        let path = json_path_at_offset(text, offset).unwrap();
        assert_eq!(path, vec!["terminal", "terminalFontSize"]);
    }

    #[test]
    fn json_path_at_offset_outside_any_pair_is_none() {
        let text = "{\"a\": 1}";
        assert!(json_path_at_offset(text, 0).is_none());
    }

    #[test]
    fn line_col_at_counts_lines() {
        let text = "{\n  \"a\": 1,\n  \"b\": 2\n}";
        let offset = text.find("\"b\"").unwrap();
        assert_eq!(line_col_at(text, offset), (3, 3));
    }
}
