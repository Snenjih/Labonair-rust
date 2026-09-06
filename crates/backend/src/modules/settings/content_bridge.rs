//! `impl From<&SettingsContent> for Preferences` (T19-001 Anweisung #6).
//!
//! Lets the rest of the app keep reading the flat, concretely-typed
//! [`Preferences`] struct unchanged while the new [`SettingsContent`] tree
//! (`labonair-settings-content`) becomes the thing actually persisted/merged
//! from `T19-002` onward. Every field is read through a fully-populated
//! `SettingsContent::defaults()` merged with the caller's tree, so a `None`
//! anywhere in `content` still yields the historically-correct `Preferences`
//! default rather than a Rust `Default::default()` (which would not
//! necessarily match — e.g. `CursorStyle::default()` is `Block`, but
//! `Preferences`' historical default is `Bar`).
//!
//! `bar_item_placements` / `bar_layout_migrated` have no `SettingsContent`
//! counterpart (the legacy bar-item blob is superseded by
//! `personalization.status_bar_item_placements`, see `migrations.rs`) — those
//! two fields fall back to `Preferences::default()`'s values. Keybindings
//! used to be a third such field (`keybinds`); T19-008 moved them into their
//! own `keymap.json` and removed the field entirely.

use labonair_settings_content::{
    ai, appearance, general, hosts, terminal, workspace, MergeFrom, SettingsContent,
};

use super::preferences::{CursorStyle, Preferences, StartupTab, ThemePref};

fn theme_pref(v: general::ThemePref) -> ThemePref {
    match v {
        general::ThemePref::System => ThemePref::System,
        general::ThemePref::Light => ThemePref::Light,
        general::ThemePref::Dark => ThemePref::Dark,
    }
}

fn startup_tab(v: general::StartupTab) -> StartupTab {
    match v {
        general::StartupTab::Terminal => StartupTab::Terminal,
        general::StartupTab::Empty => StartupTab::Empty,
    }
}

fn cursor_style(v: terminal::CursorStyle) -> CursorStyle {
    match v {
        terminal::CursorStyle::Block => CursorStyle::Block,
        terminal::CursorStyle::Underline => CursorStyle::Underline,
        terminal::CursorStyle::Bar => CursorStyle::Bar,
    }
}

fn palette_search_mode(v: workspace::PaletteSearchMode) -> super::preferences::PaletteSearchMode {
    use super::preferences::PaletteSearchMode as P;
    match v {
        workspace::PaletteSearchMode::Contains => P::Contains,
        workspace::PaletteSearchMode::StartsWith => P::StartsWith,
        workspace::PaletteSearchMode::Fuzzy => P::Fuzzy,
    }
}

impl From<&SettingsContent> for Preferences {
    fn from(content: &SettingsContent) -> Self {
        let mut merged = SettingsContent::defaults();
        merged.merge_from(content);

        let legacy = Preferences::default();

        let g = merged.general;
        let a: appearance::AppearanceContent = merged.appearance;
        let t = merged.terminal;
        let e = merged.editor;
        let f = merged.file_manager;
        let c = merged.connections;
        let h: hosts::HostsContent = merged.hosts;
        let w = merged.workspace;
        let ai: ai::AiContent = merged.ai;
        let mcp = merged.mcp;
        let p = merged.personalization;

        Preferences {
            theme: g.theme.map(theme_pref).unwrap_or_default(),
            restore_window_state: g.restore_window_state.unwrap_or_default(),
            default_startup_tab: g.default_startup_tab.map(startup_tab).unwrap_or_default(),
            startup_terminal_count: g.startup_terminal_count.unwrap_or_default(),
            autostart: g.autostart.unwrap_or_default(),
            credential_encryption: g.credential_encryption.unwrap_or_default(),
            notify_on_errors: g.notify_on_errors.unwrap_or_default(),
            confirm_quit_with_ssh: g.confirm_quit_with_ssh.unwrap_or_default(),
            check_for_updates: g.check_for_updates.unwrap_or_default(),
            session_restore: g.session_restore.unwrap_or_default(),

            app_theme: a.app_theme.unwrap_or_default(),
            icon_theme: a.icon_theme.unwrap_or_default(),
            theme_variant_overrides: a.theme_variant_overrides.unwrap_or_default(),
            app_font_size: a.app_font_size.unwrap_or_default(),
            app_line_height: a.app_line_height.unwrap_or_default(),
            app_font_family: a.app_font_family.unwrap_or_default(),
            reduce_motion: a.reduce_motion.unwrap_or_default(),
            app_corner_radius: a.app_corner_radius.unwrap_or_default(),
            background_image: a.background_image.unwrap_or_default(),
            background_opacity: a.background_opacity.unwrap_or_default(),
            background_blur: a.background_blur.unwrap_or_default(),
            background_tint_color: a.background_tint_color.unwrap_or_default(),
            background_tint_opacity: a.background_tint_opacity.unwrap_or_default(),
            tabs_location: a.tabs_location.unwrap_or_default(),
            sidebar_tab_info_line: a.sidebar_tab_info_line.unwrap_or_default(),
            sidebar_group_by_folder: a.sidebar_group_by_folder.unwrap_or_default(),
            sidebar_group_single_tabs: a.sidebar_group_single_tabs.unwrap_or_default(),
            bar_item_placements: legacy.bar_item_placements,
            bar_layout_migrated: legacy.bar_layout_migrated,
            badges_always_visible: a.badges_always_visible.unwrap_or_default(),
            titlebars_icons_position: a.titlebars_icons_position.unwrap_or_default(),
            zen_mode_show_header: a.zen_mode_show_header.unwrap_or_default(),
            zen_mode_show_statusbar: a.zen_mode_show_statusbar.unwrap_or_default(),

            status_bar_show_explorer_button: p.status_bar_show_explorer_button.unwrap_or_default(),
            status_bar_show_snippets_button: p.status_bar_show_snippets_button.unwrap_or_default(),
            status_bar_show_source_control_button: p
                .status_bar_show_source_control_button
                .unwrap_or_default(),
            status_bar_show_tabs_button: p.status_bar_show_tabs_button.unwrap_or_default(),
            status_bar_show_cwd_breadcrumb: p.status_bar_show_cwd_breadcrumb.unwrap_or_default(),
            status_bar_show_preview_url: p.status_bar_show_preview_url.unwrap_or_default(),
            status_bar_show_ai_controls: p.status_bar_show_ai_controls.unwrap_or_default(),

            sidebar_position: w.sidebar_position.unwrap_or_default(),
            sidebar_open: w.sidebar_open.unwrap_or_default(),
            sidebar_active_panel: w.sidebar_active_panel.unwrap_or_default(),
            sidebar_right_open: w.sidebar_right_open.unwrap_or_default(),
            sidebar_right_active_panel: w.sidebar_right_active_panel.unwrap_or_default(),
            sidebar_width: w.sidebar_width.unwrap_or_default(),
            sidebar_right_width: w.sidebar_right_width.unwrap_or_default(),
            dock_layout: w.dock_layout.unwrap_or_default(),
            hm_layout: h.layout.unwrap_or_default(),
            hm_sort: h.sort.unwrap_or_default(),
            hm_card_scale: h.card_scale.unwrap_or_default(),

            terminal_shell: t.terminal_shell.unwrap_or_default(),
            terminal_default_path: t.terminal_default_path.unwrap_or_default(),
            new_tab_inherits_cwd: t.new_tab_inherits_cwd.unwrap_or_default(),
            confirm_close_terminal_tab: t.confirm_close_terminal_tab.unwrap_or_default(),
            terminal_font_family: t.terminal_font_family.unwrap_or_default(),
            terminal_font_size: t.terminal_font_size.unwrap_or_default(),
            terminal_font_weight: t.terminal_font_weight.unwrap_or_default(),
            terminal_letter_spacing: t.terminal_letter_spacing.unwrap_or_default(),
            terminal_line_height: t.terminal_line_height.unwrap_or_default(),
            terminal_scrollback: t.terminal_scrollback.unwrap_or_default(),
            session_scrollback_lines: t.session_scrollback_lines.unwrap_or_default(),
            scrollback_max_size_mb: t.scrollback_max_size_mb.unwrap_or_default(),
            scrollback_retention_days: t.scrollback_retention_days.unwrap_or_default(),
            terminal_cursor_style: t
                .terminal_cursor_style
                .map(cursor_style)
                .unwrap_or_default(),
            terminal_cursor_blink: t.terminal_cursor_blink.unwrap_or_default(),
            terminal_cursor_blink_interval: t.terminal_cursor_blink_interval.unwrap_or_default(),
            terminal_copy_on_select: t.terminal_copy_on_select.unwrap_or_default(),
            terminal_right_click_pastes: t.terminal_right_click_pastes.unwrap_or_default(),
            terminal_word_separator: t.terminal_word_separator.unwrap_or_default(),
            terminal_scroll_sensitivity: t.terminal_scroll_sensitivity.unwrap_or_default(),
            terminal_fast_scroll_modifier: t.terminal_fast_scroll_modifier.unwrap_or_default(),
            terminal_show_pane_header: t.terminal_show_pane_header.unwrap_or_default(),
            terminal_show_pane_footer: t.terminal_show_pane_footer.unwrap_or_default(),
            terminal_use_webgl: t.terminal_use_webgl.unwrap_or_default(),
            terminal_composer_enabled: t.terminal_composer_enabled.unwrap_or_default(),
            terminal_composer_history_popup: t.terminal_composer_history_popup.unwrap_or_default(),
            terminal_composer_argument_completion: t
                .terminal_composer_argument_completion
                .unwrap_or_default(),
            terminal_blocks_enabled: t.terminal_blocks_enabled.unwrap_or_default(),
            terminal_blocks_auto_collapse_on_alt_screen: t
                .terminal_blocks_auto_collapse_on_alt_screen
                .unwrap_or_default(),
            terminal_bell: t.terminal_bell.unwrap_or_default(),
            terminal_opacity: t.terminal_opacity.unwrap_or_default(),

            editor_font_family: e.editor_font_family.unwrap_or_default(),
            editor_font_size: e.editor_font_size.unwrap_or_default(),
            editor_line_height: e.editor_line_height.unwrap_or_default(),
            editor_tab_size: e.editor_tab_size.unwrap_or_default(),
            editor_word_wrap: e.editor_word_wrap.unwrap_or_default(),
            editor_line_numbers: e.editor_line_numbers.unwrap_or_default(),
            editor_relative_line_numbers: e.editor_relative_line_numbers.unwrap_or_default(),
            editor_indent_with_tabs: e.editor_indent_with_tabs.unwrap_or_default(),
            editor_format_on_save: e.editor_format_on_save.unwrap_or_default(),
            editor_trim_trailing_whitespace: e.editor_trim_trailing_whitespace.unwrap_or_default(),
            editor_insert_final_newline: e.editor_insert_final_newline.unwrap_or_default(),
            editor_bracket_matching: e.editor_bracket_matching.unwrap_or_default(),
            editor_show_cursor_position: e.editor_show_cursor_position.unwrap_or_default(),
            editor_show_selection_stats: e.editor_show_selection_stats.unwrap_or_default(),
            editor_show_outline: e.editor_show_outline.unwrap_or_default(),
            editor_indentation_guides: e.editor_indentation_guides.unwrap_or_default(),
            editor_auto_save: e.editor_auto_save.unwrap_or_default(),
            editor_auto_save_delay: e.editor_auto_save_delay.unwrap_or_default(),
            editor_autocomplete_debounce_ms: e.editor_autocomplete_debounce_ms.unwrap_or_default(),
            editor_max_file_size_mb: e.editor_max_file_size_mb.unwrap_or_default(),
            editor_vim_mode: e.editor_vim_mode.unwrap_or_default(),
            editor_theme: e.editor_theme.unwrap_or_default(),

            sftp_show_hidden_files: f.sftp_show_hidden_files.unwrap_or_default(),
            sftp_show_up_folder: f.sftp_show_up_folder.unwrap_or_default(),
            explorer_show_hidden_by_default: f.explorer_show_hidden_by_default.unwrap_or_default(),
            sftp_column_size: f.sftp_column_size.unwrap_or_default(),
            sftp_column_modified: f.sftp_column_modified.unwrap_or_default(),
            sftp_column_permissions: f.sftp_column_permissions.unwrap_or_default(),
            sftp_column_type: f.sftp_column_type.unwrap_or_default(),
            sftp_remote_edit_show_transfers: f.sftp_remote_edit_show_transfers.unwrap_or_default(),
            sftp_max_remote_file_size_mb: f.sftp_max_remote_file_size_mb.unwrap_or_default(),
            sftp_font_size: f.sftp_font_size.unwrap_or_default(),
            sftp_max_concurrent_transfers: f.sftp_max_concurrent_transfers.unwrap_or_default(),
            sftp_default_conflict_resolution: f
                .sftp_default_conflict_resolution
                .unwrap_or_default(),
            sftp_chunk_size_kb: f.sftp_chunk_size_kb.unwrap_or_default(),
            sftp_on_folder_file_error: f.sftp_on_folder_file_error.unwrap_or_default(),

            host_ping_interval: c.host_ping_interval.unwrap_or_default(),
            ssh_connect_timeout_secs: c.ssh_connect_timeout_secs.unwrap_or_default(),
            ssh_auto_reconnect: c.ssh_auto_reconnect.unwrap_or_default(),
            ssh_auto_reconnect_delay: c.ssh_auto_reconnect_delay.unwrap_or_default(),
            ssh_auto_reconnect_max_attempts: c.ssh_auto_reconnect_max_attempts.unwrap_or_default(),
            explorer_remote_poll_interval: c.explorer_remote_poll_interval.unwrap_or_default(),
            explorer_auto_reconnect: c.explorer_auto_reconnect.unwrap_or_default(),
            explorer_idle_session_timeout_min: c
                .explorer_idle_session_timeout_min
                .unwrap_or_default(),
            explorer_max_idle_sessions: c.explorer_max_idle_sessions.unwrap_or_default(),
            explorer_max_cached_remote_scopes: c
                .explorer_max_cached_remote_scopes
                .unwrap_or_default(),

            command_palette_search_mode: w
                .command_palette_search_mode
                .map(palette_search_mode)
                .unwrap_or_default(),
            command_palette_show_recent: w.command_palette_show_recent.unwrap_or_default(),
            command_palette_blur: w.command_palette_blur.unwrap_or_default(),
            command_palette_opacity: w.command_palette_opacity.unwrap_or_default(),
            command_palette_position: w.command_palette_position.unwrap_or_default(),
            command_palette_animation: w.command_palette_animation.unwrap_or_default(),
            command_palette_history_size: w.command_palette_history_size.unwrap_or_default(),
            command_palette_close_on_overlay_click: w
                .command_palette_close_on_overlay_click
                .unwrap_or_default(),

            git_status_poll_interval_ms: w.git_status_poll_interval_ms.unwrap_or_default(),

            ai_enabled: ai.ai_enabled.unwrap_or_default(),
            ai_max_agent_steps: ai.ai_max_agent_steps.unwrap_or_default(),
            ai_terminal_context_lines: ai.ai_terminal_context_lines.unwrap_or_default(),
            ai_temperature: ai.ai_temperature.unwrap_or_default(),
            ai_warn_destructive_commands: ai.ai_warn_destructive_commands.unwrap_or_default(),
            ai_auto_open_mini_on_send: ai.ai_auto_open_mini_on_send.unwrap_or_default(),
            ai_notify_on_headless_command: ai.ai_notify_on_headless_command.unwrap_or_default(),
            ai_shell_max_timeout_secs: ai.ai_shell_max_timeout_secs.unwrap_or_default(),
            ai_shell_max_output_kb: ai.ai_shell_max_output_kb.unwrap_or_default(),
            default_model_id: ai.default_model_id.unwrap_or_default(),
            custom_instructions: ai.custom_instructions.unwrap_or_default(),
            autocomplete_enabled: ai.autocomplete_enabled.unwrap_or_default(),
            autocomplete_provider: ai.autocomplete_provider.unwrap_or_default(),
            autocomplete_model_id: ai.autocomplete_model_id.unwrap_or_default(),
            lmstudio_base_url: ai.lmstudio_base_url.unwrap_or_default(),
            lmstudio_chat_model_id: ai.lmstudio_chat_model_id.unwrap_or_default(),
            openai_compatible_base_url: ai.openai_compatible_base_url.unwrap_or_default(),
            openai_compatible_model_id: ai.openai_compatible_model_id.unwrap_or_default(),
            mlx_base_url: ai.mlx_base_url.unwrap_or_default(),
            mlx_chat_model_id: ai.mlx_chat_model_id.unwrap_or_default(),
            ollama_base_url: ai.ollama_base_url.unwrap_or_default(),
            ollama_chat_model_id: ai.ollama_chat_model_id.unwrap_or_default(),

            mcp_bridge_enabled: mcp.bridge_enabled.unwrap_or_default(),
            mcp_bridge_port: mcp.bridge_port.unwrap_or_default(),
            mcp_max_command_timeout_secs: mcp.max_command_timeout_secs.unwrap_or_default(),
            mcp_auto_revoke_minutes: mcp.auto_revoke_minutes.unwrap_or_default(),
            mcp_notify_on_activity: mcp.notify_on_activity.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_to_the_historical_preferences_defaults() {
        let content = SettingsContent::defaults();
        let prefs = Preferences::from(&content);
        assert_eq!(prefs, Preferences::default());
    }

    #[test]
    fn a_single_override_survives_the_bridge() {
        let mut content = SettingsContent::defaults();
        content.terminal.terminal_font_size = Some(42);
        let prefs = Preferences::from(&content);
        assert_eq!(prefs.terminal_font_size, 42);
        // Untouched fields keep their historical default.
        assert_eq!(
            prefs.editor_font_size,
            Preferences::default().editor_font_size
        );
    }

    #[test]
    fn missing_fields_fall_back_through_settings_content_defaults() {
        // A layer with nothing set at all (e.g. a fresh empty user layer).
        let content = SettingsContent::default();
        let prefs = Preferences::from(&content);
        assert_eq!(prefs, Preferences::default());
    }
}
