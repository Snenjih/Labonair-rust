//! Explicit save policies owned by the editor.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavePolicy {
    pub auto_save: bool,
    pub auto_save_delay_ms: u64,
    pub format_on_save: bool,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
}

impl Default for SavePolicy {
    fn default() -> Self {
        Self {
            auto_save: false,
            auto_save_delay_ms: 1_000,
            format_on_save: false,
            trim_trailing_whitespace: false,
            insert_final_newline: false,
        }
    }
}

impl SavePolicy {
    pub fn normalized(self) -> Self {
        Self {
            auto_save_delay_ms: self.auto_save_delay_ms.clamp(250, 60_000),
            ..self
        }
    }
}

pub fn apply_text_policies(text: &str, policy: SavePolicy) -> String {
    let policy = policy.normalized();
    let mut lines = text
        .split_inclusive('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    if policy.trim_trailing_whitespace {
        for line in &mut lines {
            let newline = line.ends_with('\n');
            let body_len = line.len().saturating_sub(usize::from(newline));
            let body = line[..body_len].trim_end_matches([' ', '\t']);
            let mut cleaned = body.to_string();
            if newline {
                cleaned.push('\n');
            }
            *line = cleaned;
        }
    }
    let mut result = lines.concat();
    if policy.insert_final_newline && !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_policies_are_disabled_by_default() {
        let policy = SavePolicy::default();
        assert!(!policy.auto_save);
        assert!(!policy.format_on_save);
        assert_eq!(apply_text_policies("a  \n", policy), "a  \n");
    }

    #[test]
    fn whitespace_and_final_newline_are_opt_in() {
        let policy = SavePolicy {
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            ..SavePolicy::default()
        };
        assert_eq!(apply_text_policies("a  \r\nb\t", policy), "a  \r\nb\n");
    }

    #[test]
    fn autosave_delay_is_clamped() {
        assert_eq!(
            SavePolicy {
                auto_save_delay_ms: 1,
                ..SavePolicy::default()
            }
            .normalized()
            .auto_save_delay_ms,
            250
        );
        assert_eq!(
            SavePolicy {
                auto_save_delay_ms: 90_000,
                ..SavePolicy::default()
            }
            .normalized()
            .auto_save_delay_ms,
            60_000
        );
    }
}
