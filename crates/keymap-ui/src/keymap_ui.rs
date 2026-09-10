//! Native keymap management surface.
//!
//! This crate owns presentation only. The keymap domain supplies the
//! lossless document, diagnostics, effective command rows, override state, and
//! conflict data; the tab owner (`labonair-workspace`) constructs and hosts
//! [`KeymapManagementView`] as a workspace tab, supplies the composition
//! callback that opens the raw JSONC document, and drives [`reload`] when the
//! file changes on disk. No command execution, file parsing, tab lifecycle, or
//! feature state belongs here.

mod keystroke;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use gpui::{
    div, point, prelude::FluentBuilder, px, uniform_list, AnyElement, App, AppContext, ClickEvent,
    Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, Keystroke,
    ParentElement, Pixels, Point, Render, ScrollStrategy, SharedString, StatefulInteractiveElement,
    Styled, Subscription, UniformListScrollHandle, WeakEntity, Window,
};

use labonair_command_palette_core::{CommandContext, CommandDescriptor, CommandId};
use labonair_keymap::{
    adapter, file, management,
    management::{KeymapCommandRow, KeymapManagementSnapshot, OverrideState, RowFilter},
    runtime,
};
use labonair_notifications::{notification_center, Notification};
use labonair_theme::ThemeStore;
use labonair_ui_kit::{
    banner, button, disclosure, field_input, kbd_row, popover, segmented_control, ButtonSize,
    ButtonVariant, IconName, InputEvent, InputState, ListItem, Palette, SegmentVariant, Severity,
};

/// Composition callback that opens the raw user `keymap.json` document. Its
/// tab lifecycle belongs to the workspace, so the owner injects it.
pub type OpenRawCallback = Box<dyn FnMut(&mut Window, &mut App) + 'static>;

const GLOBAL_KEY: &str = "__global__";
const MAX_CHORD_KEYSTROKES: usize = 3;
// `uniform_list` measures the first item and reuses that height for every
// row, so section headers and command rows must be the same height.
const ROW_HEIGHT: f32 = 44.0;
const SECTION_HEIGHT: f32 = ROW_HEIGHT;

/// The keymap tab's view state.
pub struct KeymapManagementView {
    theme: Entity<ThemeStore>,
    descriptors: Vec<CommandDescriptor>,
    snapshot: Option<KeymapManagementSnapshot>,
    load_error: Option<String>,
    open_raw: Option<OpenRawCallback>,

    // Browsing / filtering / keyboard navigation.
    filter: String,
    row_filter: RowFilter,
    collapsed_sections: HashSet<String>,
    selected: usize,
    list_scroll: UniformListScrollHandle,

    // The rebind editor, anchored as a popover while open.
    editor: Option<BindingEditor>,
    capture_subscription: Option<Subscription>,

    // Live external-change handling.
    external_change_pending: bool,
    last_self_write: Option<Instant>,

    focus: FocusHandle,
}

/// One row of the flattened, virtualized list.
enum VisibleRow {
    Section {
        name: String,
        collapsed: bool,
        count: usize,
    },
    /// Index into the matched-rows vector built alongside this one.
    Command(usize),
}

struct BindingEditor {
    command: CommandId,
    action: String,
    title: String,
    /// Portable context this binding is scoped to (`None` = global).
    context: Option<CommandContext>,
    /// Contexts the command allows; empty means global-only.
    available_contexts: Vec<CommandContext>,
    /// Whether the user may still pick the scope (only when adding a binding).
    context_locked: bool,
    /// Effective chords for this command in `context` — unbound when replaced.
    original_bindings: Vec<String>,
    /// The command ships a default for `context` that could be restored.
    resettable: bool,
    anchor: Point<Pixels>,
    recording: bool,
    captured: Vec<Keystroke>,
    text_input: Option<Entity<InputState>>,
    error: Option<String>,
    saving: bool,
    _text_subscription: Option<Subscription>,
}

impl Focusable for KeymapManagementView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl KeymapManagementView {
    pub fn new(
        theme: Entity<ThemeStore>,
        descriptors: Vec<CommandDescriptor>,
        open_raw: OpenRawCallback,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            theme,
            descriptors,
            snapshot: None,
            load_error: None,
            open_raw: Some(open_raw),
            filter: String::new(),
            row_filter: RowFilter::All,
            collapsed_sections: HashSet::new(),
            selected: 0,
            list_scroll: UniformListScrollHandle::new(),
            editor: None,
            capture_subscription: None,
            external_change_pending: false,
            last_self_write: None,
            focus: cx.focus_handle(),
        };
        view.load_snapshot(cx);
        view
    }

    // ── loading ───────────────────────────────────────────────────────────

    fn load_snapshot(&mut self, cx: &mut Context<Self>) {
        let descriptors = self.descriptors.clone();
        let load = cx.background_executor().spawn(async move {
            let known_actions = management::known_actions(descriptors.iter());
            let document = file::read_user_keymap_document(&known_actions)?;
            let runtime = adapter::load_descriptors(descriptors.iter());
            Ok::<_, String>(management::snapshot(descriptors.iter(), &runtime, document))
        });
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| match load.await {
            Ok(snapshot) => {
                let _ = entity.update(cx, |view, cx| {
                    view.publish_diagnostics(&snapshot, cx);
                    view.snapshot = Some(snapshot);
                    view.load_error = None;
                    view.external_change_pending = false;
                    view.clamp_selection();
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(cx, |view, cx| {
                    view.publish_error(
                        "Keymap could not be loaded",
                        "The active keymap was not changed.",
                        error.clone(),
                        cx,
                    );
                    view.load_error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Re-read `keymap.json` after an external change (raw editor, another
    /// editor, a sync). Deferred while a rebind is in progress so an in-flight
    /// edit is never discarded; a reload that immediately follows this view's
    /// own write is a no-op (the write path already refreshed the snapshot).
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        if self
            .last_self_write
            .is_some_and(|at| at.elapsed() < Duration::from_millis(1500))
        {
            return;
        }
        if self.editor.is_some() || self.capture_subscription.is_some() {
            self.external_change_pending = true;
            cx.notify();
            return;
        }
        self.load_snapshot(cx);
    }

    fn publish_error(
        &self,
        title: &'static str,
        summary: &'static str,
        details: String,
        cx: &mut Context<Self>,
    ) {
        if !cx.has_global::<labonair_notifications::GlobalNotificationCenter>() {
            return;
        }
        notification_center(cx).update(cx, |center, cx| {
            center.push(
                Notification::error(title, summary)
                    .details(details.clone())
                    .dedupe_key(format!("keymap:{title}:{details}")),
                cx,
            );
        });
    }

    fn publish_diagnostics(&mut self, snapshot: &KeymapManagementSnapshot, cx: &mut Context<Self>) {
        if snapshot.issues.is_empty() {
            return;
        }
        let details = snapshot
            .issues
            .iter()
            .map(|issue| format!("line {}: {}", issue.line, issue.message))
            .collect::<Vec<_>>()
            .join("\n");
        let error_count = snapshot
            .issues
            .iter()
            .filter(|issue| matches!(issue.severity, file::Severity::Error))
            .count();
        let summary = format!(
            "{} error(s), {} warning(s) found in keymap.json.",
            error_count,
            snapshot.issues.len() - error_count
        );
        if !cx.has_global::<labonair_notifications::GlobalNotificationCenter>() {
            return;
        }
        notification_center(cx).update(cx, |center, cx| {
            let notification = if error_count > 0 {
                Notification::error("Keymap diagnostics", summary)
            } else {
                Notification::warning("Keymap diagnostics", summary)
            };
            center.push(
                notification
                    .details(details.clone())
                    .dedupe_key(format!("keymap-diagnostics:{details}")),
                cx,
            );
        });
    }

    // ── list model ────────────────────────────────────────────────────────

    fn is_narrowed(&self) -> bool {
        !self.filter.trim().is_empty() || self.row_filter != RowFilter::All
    }

    fn build_visible(
        &self,
        snapshot: &KeymapManagementSnapshot,
    ) -> (Vec<VisibleRow>, Vec<KeymapCommandRow>) {
        let matched: Vec<KeymapCommandRow> = snapshot
            .search_filtered(&self.filter, self.row_filter)
            .into_iter()
            .cloned()
            .collect();

        let force_open = self.is_narrowed();
        let mut section_order: Vec<String> = Vec::new();
        let mut grouped: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (index, row) in matched.iter().enumerate() {
            if !grouped.contains_key(&row.section) {
                section_order.push(row.section.clone());
            }
            grouped.entry(row.section.clone()).or_default().push(index);
        }

        let mut visible = Vec::new();
        for section in section_order {
            let indices = &grouped[&section];
            let collapsed = !force_open && self.collapsed_sections.contains(&section);
            visible.push(VisibleRow::Section {
                name: section.clone(),
                collapsed,
                count: indices.len(),
            });
            if !collapsed {
                visible.extend(indices.iter().map(|index| VisibleRow::Command(*index)));
            }
        }
        (visible, matched)
    }

    fn clamp_selection(&mut self) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let (visible, _) = self.build_visible(snapshot);
        if visible.is_empty() {
            self.selected = 0;
            return;
        }
        if self.selected >= visible.len()
            || !matches!(visible[self.selected], VisibleRow::Command(_))
        {
            self.selected = visible
                .iter()
                .position(|row| matches!(row, VisibleRow::Command(_)))
                .unwrap_or(0);
        }
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let (visible, _) = self.build_visible(&snapshot);
        if visible.is_empty() {
            return;
        }
        let mut index = self.selected.min(visible.len() - 1) as isize;
        loop {
            index += delta;
            if index < 0 || index as usize >= visible.len() {
                return;
            }
            if matches!(visible[index as usize], VisibleRow::Command(_)) {
                self.selected = index as usize;
                self.list_scroll
                    .scroll_to_item(self.selected, ScrollStrategy::Center);
                cx.notify();
                return;
            }
        }
    }

    fn toggle_section(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.collapsed_sections.remove(name) {
            self.collapsed_sections.insert(name.to_string());
        }
        self.clamp_selection();
        cx.notify();
    }

    // ── keyboard ──────────────────────────────────────────────────────────

    fn on_key(&mut self, event: &gpui::KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let key = keystroke.key.as_str();
        let modified = keystroke.modifiers.platform
            || keystroke.modifiers.control
            || keystroke.modifiers.alt
            || keystroke.modifiers.function;

        if self.editor.is_some() {
            if key == "escape" {
                self.close_editor(cx);
                cx.stop_propagation();
            }
            return;
        }

        match key {
            "down" => self.move_selection(1, cx),
            "up" => self.move_selection(-1, cx),
            "enter" => self.activate_selected(cx),
            "escape" if !self.filter.is_empty() => {
                self.filter.clear();
                self.clamp_selection();
                cx.notify();
            }
            "backspace" if !modified => {
                self.filter.pop();
                self.clamp_selection();
                cx.notify();
            }
            _ if !modified => {
                let typed = keystroke
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(char::is_control))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let Some(text) = typed {
                    self.filter.push_str(&text);
                    self.selected = 0;
                    self.clamp_selection();
                    cx.notify();
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    fn activate_selected(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let (visible, matched) = self.build_visible(&snapshot);
        let Some(row) = visible.get(self.selected) else {
            return;
        };
        match row {
            VisibleRow::Section { name, .. } => {
                let name = name.clone();
                self.toggle_section(&name, cx);
            }
            VisibleRow::Command(index) => {
                if let Some(command_row) = matched.get(*index).cloned() {
                    let binding_index = (!command_row.effective_bindings.is_empty()).then_some(0);
                    let anchor = point(px(320.0), px(180.0));
                    self.begin_edit(&command_row, binding_index, anchor, &snapshot, cx);
                }
            }
        }
    }

    // ── editor lifecycle ──────────────────────────────────────────────────

    fn begin_edit(
        &mut self,
        row: &KeymapCommandRow,
        binding_index: Option<usize>,
        anchor: Point<Pixels>,
        snapshot: &KeymapManagementSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.stop_recording();
        let _ = snapshot;

        let (context, captured) = match binding_index {
            Some(index) => {
                let binding = &row.effective_bindings[index];
                let context = binding.context.as_deref().and_then(context_from_label);
                let captured = binding
                    .keystrokes
                    .split_whitespace()
                    .filter_map(|token| Keystroke::parse(token).ok())
                    .collect();
                (context, captured)
            }
            None => (row.contexts.first().copied(), Vec::new()),
        };
        let context_locked = binding_index.is_some() || row.contexts.is_empty();

        let original_bindings = row
            .effective_bindings
            .iter()
            .filter(|binding| binding.context.as_deref().and_then(context_from_label) == context)
            .map(|binding| binding.keystrokes.clone())
            .collect::<Vec<_>>();

        let resettable = matches!(
            management::row_override_state(row),
            OverrideState::Customized | OverrideState::Unbound
        ) && row
            .default_bindings
            .iter()
            .any(|default| default.context == context);

        self.editor = Some(BindingEditor {
            command: row.command,
            action: row.command.action_name().to_string(),
            title: row.title.clone(),
            context,
            available_contexts: row.contexts.clone(),
            context_locked,
            original_bindings,
            resettable,
            anchor,
            recording: false,
            captured,
            text_input: None,
            error: None,
            saving: false,
            _text_subscription: None,
        });
        cx.notify();
    }

    fn close_editor(&mut self, cx: &mut Context<Self>) {
        self.stop_recording();
        self.editor = None;
        if self.external_change_pending {
            self.load_snapshot(cx);
        }
        cx.notify();
    }

    fn start_recording(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        editor.recording = true;
        editor.captured.clear();
        editor.error = None;
        let weak: WeakEntity<Self> = cx.entity().downgrade();
        self.capture_subscription = Some(cx.intercept_keystrokes(move |event, _window, cx| {
            let keystroke = event.keystroke.clone();
            let _ = weak.update(cx, |view, cx| view.on_captured(keystroke, cx));
            cx.stop_propagation();
        }));
        cx.notify();
    }

    fn stop_recording(&mut self) {
        self.capture_subscription = None;
        if let Some(editor) = self.editor.as_mut() {
            editor.recording = false;
        }
    }

    fn on_captured(&mut self, keystroke: Keystroke, cx: &mut Context<Self>) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if !editor.recording {
            return;
        }
        if keystroke.key == "escape" {
            self.stop_recording();
            cx.notify();
            return;
        }
        if keystroke::is_modifier_only(&keystroke) {
            cx.notify();
            return;
        }
        editor.captured.push(keystroke);
        if editor.captured.len() >= MAX_CHORD_KEYSTROKES {
            self.stop_recording();
        }
        cx.notify();
    }

    fn toggle_text_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_recording();
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if editor.text_input.is_some() {
            editor.text_input = None;
            editor._text_subscription = None;
            cx.notify();
            return;
        }
        let seed = keystroke::to_binding_string(&editor.captured);
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. cmd-k cmd-s")
                .default_value(seed)
        });
        let subscription = cx.subscribe(&input, |view, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                view.commit_edit(cx);
            }
        });
        input.update(cx, |state, cx| state.focus(window, cx));
        editor.text_input = Some(input);
        editor._text_subscription = Some(subscription);
        cx.notify();
    }

    fn candidate_binding(&self, cx: &App) -> String {
        let Some(editor) = &self.editor else {
            return String::new();
        };
        if let Some(input) = &editor.text_input {
            input.read(cx).value().trim().to_string()
        } else {
            keystroke::to_binding_string(&editor.captured)
        }
    }

    fn set_context(&mut self, context: Option<CommandContext>, cx: &mut Context<Self>) {
        if let Some(editor) = self.editor.as_mut() {
            if editor.context_locked {
                return;
            }
            editor.context = context;
            editor.error = None;
        }
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        self.write_edit(false, cx);
    }

    fn unbind_edit(&mut self, cx: &mut Context<Self>) {
        self.write_edit(true, cx);
    }

    fn write_edit(&mut self, unbind: bool, cx: &mut Context<Self>) {
        self.stop_recording();
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        if self
            .editor
            .as_ref()
            .map(|editor| editor.saving)
            .unwrap_or(true)
        {
            return;
        }

        let candidate = if unbind {
            String::new()
        } else {
            self.candidate_binding(cx)
        };
        let Some(editor) = self.editor.as_mut() else {
            return;
        };

        if !unbind {
            if candidate.is_empty() {
                editor.error = Some("Press a key combination, or type one.".to_string());
                cx.notify();
                return;
            }
            for token in candidate.split_whitespace() {
                if let Err(error) = Keystroke::parse(token) {
                    editor.error = Some(format!("Not a valid keystroke: `{token}` ({error})."));
                    cx.notify();
                    return;
                }
            }
            let context_name = editor.context.map(runtime::context_name);
            if let Some(conflict) = snapshot.conflict(editor.command, context_name, &candidate) {
                editor.error = Some(format!(
                    "Already bound to “{}” in {}.",
                    conflict.title,
                    conflict
                        .context
                        .as_deref()
                        .unwrap_or(runtime::GLOBAL_CONTEXT_LABEL)
                ));
                cx.notify();
                return;
            }
        }

        let source = snapshot.document.source.clone();
        let context_name = editor
            .context
            .map(runtime::context_name)
            .map(str::to_string);
        let old_bindings = editor.original_bindings.clone();
        let action = editor.action.clone();
        let replacement = (!candidate.is_empty()).then(|| candidate.clone());

        let updated = match file::append_user_binding_override(
            &source,
            context_name.as_deref(),
            &old_bindings,
            &action,
            replacement.as_deref(),
        ) {
            Ok(updated) => updated,
            Err(error) => {
                editor.error = Some(error);
                cx.notify();
                return;
            }
        };

        editor.saving = true;
        editor.error = None;
        self.persist(updated, cx);
    }

    fn reset_to_default(&mut self, cx: &mut Context<Self>) {
        self.stop_recording();
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if editor.saving {
            return;
        }
        let context_name = editor
            .context
            .map(runtime::context_name)
            .map(str::to_string);
        let action = editor.action.clone();
        let defaults: Vec<String> = self
            .descriptors
            .iter()
            .find(|descriptor| descriptor.id == editor.command)
            .map(|descriptor| {
                descriptor
                    .default_bindings
                    .iter()
                    .filter(|default| {
                        default
                            .context
                            .map(runtime::context_name)
                            .map(str::to_string)
                            == context_name
                    })
                    .map(|default| default.keystrokes.clone())
                    .collect()
            })
            .unwrap_or_default();

        match file::remove_user_binding_override(
            &snapshot.document.source,
            context_name.as_deref(),
            &action,
            &defaults,
        ) {
            Ok(updated) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.saving = true;
                    editor.error = None;
                }
                self.persist(updated, cx);
            }
            Err(error) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.error = Some(error.clone());
                }
                self.publish_error(
                    "Could not reset the binding",
                    "Edit keymap.json directly to change it.",
                    error,
                    cx,
                );
                cx.notify();
            }
        }
    }

    fn persist(&mut self, updated: String, cx: &mut Context<Self>) {
        let save = cx
            .background_executor()
            .spawn(async move { file::save_user_keymap_document(&updated) });
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| match save.await {
            Ok(_) => {
                let _ = entity.update(cx, |view, cx| {
                    view.last_self_write = Some(Instant::now());
                    view.editor = None;
                    view.capture_subscription = None;
                    view.load_snapshot(cx);
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(cx, |view, cx| {
                    if let Some(editor) = view.editor.as_mut() {
                        editor.saving = false;
                        editor.error = Some(format!("Could not save keymap.json: {error}"));
                    }
                    view.publish_error(
                        "Could not save keymap.json",
                        "The keyboard binding was not saved.",
                        error,
                        cx,
                    );
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn open_raw_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(callback) = self.open_raw.as_mut() {
            callback(window, cx);
        }
    }
}

// ── free rendering helpers (uniform_list closure only has `&mut App`) ──────

fn chord_chips(chord: &str) -> Vec<String> {
    chord
        .split_whitespace()
        .flat_map(labonair_keymap::keystroke_tokens)
        .collect()
}

fn context_from_label(label: &str) -> Option<CommandContext> {
    match label {
        "Editor" => Some(CommandContext::Editor),
        "Terminal" => Some(CommandContext::Terminal),
        "Sftp" | "SFTP" => Some(CommandContext::Sftp),
        "Home" => Some(CommandContext::Home),
        "SshTerminal" | "SSH Terminal" => Some(CommandContext::SshTerminal),
        _ => None,
    }
}

fn context_label(context: Option<CommandContext>) -> &'static str {
    context.map_or(runtime::GLOBAL_CONTEXT_LABEL, runtime::context_label)
}

fn render_command_row(
    view: &Entity<KeymapManagementView>,
    row: &KeymapCommandRow,
    matched_index: usize,
    selected: bool,
    conflicted: bool,
    palette: Palette,
) -> AnyElement {
    let state = management::row_override_state(row);
    let contexts: Vec<String> = {
        let mut seen = Vec::new();
        for binding in &row.effective_bindings {
            let label =
                context_label(binding.context.as_deref().and_then(context_from_label)).to_string();
            if !seen.contains(&label) {
                seen.push(label);
            }
        }
        if seen.is_empty() {
            seen.push(
                row.contexts
                    .first()
                    .copied()
                    .map(runtime::context_label)
                    .unwrap_or(runtime::GLOBAL_CONTEXT_LABEL)
                    .to_string(),
            );
        }
        seen
    };

    let trailing: AnyElement = if row.effective_bindings.is_empty() {
        add_binding_button(view, row, matched_index, palette)
    } else {
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(
                (0..row.effective_bindings.len())
                    .map(|index| binding_button(view, row, matched_index, index, palette)),
            )
            .child(add_binding_button(view, row, matched_index, palette))
            .into_any_element()
    };

    let subtitle = contexts.join(", ");
    let status: Option<AnyElement> = match state {
        OverrideState::Customized => Some(
            div()
                .text_size(px(9.0))
                .text_color(palette.info)
                .child("Modified")
                .into_any_element(),
        ),
        OverrideState::Unbound => Some(
            div()
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child("Unbound")
                .into_any_element(),
        ),
        OverrideState::Default => None,
    };

    let view_for_click = view.clone();
    let command = row.command;
    div()
        .h(px(ROW_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .child(
            ListItem::new(
                ("keymap-row", matched_index),
                palette.fg,
                palette.muted,
                palette.selected_fill,
            )
            .selected(selected)
            .icon(if conflicted {
                IconName::Warning
            } else {
                IconName::Command
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_size(px(12.0))
                            .child(SharedString::from(row.title.clone()))
                            .children(status),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(if conflicted {
                                palette.warning
                            } else {
                                palette.muted
                            })
                            .child(SharedString::from(if conflicted {
                                format!("{subtitle} · overridden by another command")
                            } else {
                                subtitle
                            })),
                    ),
            )
            .trailing(trailing)
            .on_click(move |event: &ClickEvent, _window, cx| {
                let position = event.position();
                view_for_click.update(cx, |this, cx| {
                    if let Some(snapshot) = this.snapshot.clone() {
                        if let Some(command_row) = snapshot
                            .rows
                            .iter()
                            .find(|candidate| candidate.command == command)
                            .cloned()
                        {
                            let binding_index =
                                (!command_row.effective_bindings.is_empty()).then_some(0);
                            this.begin_edit(&command_row, binding_index, position, &snapshot, cx);
                        }
                    }
                });
            }),
        )
        .into_any_element()
}

fn binding_button(
    view: &Entity<KeymapManagementView>,
    row: &KeymapCommandRow,
    matched_index: usize,
    binding_index: usize,
    palette: Palette,
) -> AnyElement {
    let chord = row.effective_bindings[binding_index].keystrokes.clone();
    let command = row.command;
    let view = view.clone();
    button(
        ("keymap-binding", matched_index * 8 + binding_index),
        palette,
        ButtonVariant::Ghost,
        ButtonSize::Xs,
    )
    .child(kbd_row(chord_chips(&chord), palette))
    .on_click(move |event: &ClickEvent, _window, cx| {
        // Don't also trigger the row's open-editor click.
        cx.stop_propagation();
        let position = event.position();
        view.update(cx, |this, cx| {
            if let Some(snapshot) = this.snapshot.clone() {
                if let Some(command_row) = snapshot
                    .rows
                    .iter()
                    .find(|candidate| candidate.command == command)
                    .cloned()
                {
                    this.begin_edit(&command_row, Some(binding_index), position, &snapshot, cx);
                }
            }
        });
    })
    .into_any_element()
}

fn add_binding_button(
    view: &Entity<KeymapManagementView>,
    row: &KeymapCommandRow,
    matched_index: usize,
    palette: Palette,
) -> AnyElement {
    let command = row.command;
    let unbound = row.effective_bindings.is_empty();
    let view = view.clone();
    button(
        ("keymap-add", matched_index),
        palette,
        if unbound {
            ButtonVariant::Outline
        } else {
            ButtonVariant::Ghost
        },
        ButtonSize::Xs,
    )
    .child(if unbound { "Add binding" } else { "+" })
    .on_click(move |event: &ClickEvent, _window, cx| {
        cx.stop_propagation();
        let position = event.position();
        view.update(cx, |this, cx| {
            if let Some(snapshot) = this.snapshot.clone() {
                if let Some(command_row) = snapshot
                    .rows
                    .iter()
                    .find(|candidate| candidate.command == command)
                    .cloned()
                {
                    this.begin_edit(&command_row, None, position, &snapshot, cx);
                }
            }
        });
    })
    .into_any_element()
}

impl KeymapManagementView {
    fn render_editor(&self, palette: Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.editor.as_ref()?;
        let candidate = self.candidate_binding(cx);

        let mut card = div()
            .w(px(320.0))
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(12.0)).text_color(palette.fg).child(
                        SharedString::from(if editor.original_bindings.is_empty() {
                            format!("Add a binding · {}", editor.title)
                        } else {
                            format!("Rebind · {}", editor.title)
                        }),
                    ))
                    .child(div().text_size(px(10.0)).text_color(palette.muted).child(
                        SharedString::from(context_label(editor.context).to_string()),
                    )),
            );

        // Context picker — only while adding, and only if the command scopes.
        if !editor.context_locked && !editor.available_contexts.is_empty() {
            let selected_key = editor
                .context
                .map(runtime::context_name)
                .unwrap_or(GLOBAL_KEY);
            let mut control = segmented_control("keymap-context", palette, selected_key)
                .variant(SegmentVariant::Outline)
                .segment(GLOBAL_KEY, runtime::GLOBAL_CONTEXT_LABEL);
            for context in &editor.available_contexts {
                control = control.segment(
                    runtime::context_name(*context),
                    runtime::context_label(*context),
                );
            }
            let available = editor.available_contexts.clone();
            card = card.child(control.on_select(cx.listener(
                move |this, key: &SharedString, _window, cx| {
                    let picked = available
                        .iter()
                        .copied()
                        .find(|context| runtime::context_name(*context) == key.as_ref());
                    this.set_context(picked, cx);
                },
            )));
        }

        // Capture surface.
        card = card.child(
            div()
                .h(px(34.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .rounded(px(5.0))
                .border_1()
                .border_color(if editor.recording {
                    palette.selected_accent
                } else {
                    palette.border
                })
                .child(if editor.recording {
                    div()
                        .text_size(px(10.0))
                        .text_color(palette.muted)
                        .child("Recording — press the keys…")
                        .into_any_element()
                } else if candidate.is_empty() {
                    div()
                        .text_size(px(10.0))
                        .text_color(palette.muted)
                        .child("No binding")
                        .into_any_element()
                } else {
                    kbd_row(chord_chips(&candidate), palette).into_any_element()
                }),
        );

        if let Some(input) = &editor.text_input {
            card = card.child(
                div()
                    .h(px(30.0))
                    .border_1()
                    .border_color(palette.border)
                    .rounded(px(5.0))
                    .child(field_input(input)),
            );
        }

        // Row of controls: record / manual toggle.
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    button(
                        "keymap-record",
                        palette,
                        if editor.recording {
                            ButtonVariant::Default
                        } else {
                            ButtonVariant::Outline
                        },
                        ButtonSize::Xs,
                    )
                    .child(if editor.recording { "Stop" } else { "Record" })
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            let recording = this
                                .editor
                                .as_ref()
                                .map(|editor| editor.recording)
                                .unwrap_or(false);
                            if recording {
                                this.stop_recording();
                            } else {
                                this.start_recording(cx);
                            }
                            cx.notify();
                        },
                    )),
                )
                .child(
                    button(
                        "keymap-manual",
                        palette,
                        ButtonVariant::Ghost,
                        ButtonSize::Xs,
                    )
                    .child(if editor.text_input.is_some() {
                        "Use recorder"
                    } else {
                        "Type manually"
                    })
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| {
                            this.toggle_text_entry(window, cx);
                        },
                    )),
                ),
        );

        if let Some(error) = &editor.error {
            card = card.child(
                div()
                    .text_size(px(10.0))
                    .text_color(palette.error)
                    .child(SharedString::from(error.clone())),
            );
        }

        // Default + reset.
        if editor.resettable {
            let defaults: Vec<String> = self
                .descriptors
                .iter()
                .find(|descriptor| descriptor.id == editor.command)
                .map(|descriptor| {
                    descriptor
                        .default_bindings
                        .iter()
                        .filter(|default| default.context == editor.context)
                        .map(|default| default.keystrokes.clone())
                        .collect()
                })
                .unwrap_or_default();
            let mut default_row = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child("Default:");
            if defaults.is_empty() {
                default_row = default_row.child("none");
            } else {
                for chord in &defaults {
                    default_row = default_row.child(kbd_row(chord_chips(chord), palette));
                }
            }
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(default_row)
                    .child(
                        button(
                            "keymap-reset",
                            palette,
                            ButtonVariant::Ghost,
                            ButtonSize::Xs,
                        )
                        .child("Reset")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.reset_to_default(cx);
                            },
                        )),
                    ),
            );
        }

        // Save / Unbind / Cancel.
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    button(
                        "keymap-save",
                        palette,
                        ButtonVariant::Default,
                        ButtonSize::Xs,
                    )
                    .child(if editor.saving { "Saving…" } else { "Save" })
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.commit_edit(cx);
                        },
                    )),
                )
                .when(!editor.original_bindings.is_empty(), |row| {
                    row.child(
                        button(
                            "keymap-unbind",
                            palette,
                            ButtonVariant::Outline,
                            ButtonSize::Xs,
                        )
                        .child("Unbind")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.unbind_edit(cx);
                            },
                        )),
                    )
                })
                .child(
                    button(
                        "keymap-cancel",
                        palette,
                        ButtonVariant::Ghost,
                        ButtonSize::Xs,
                    )
                    .child("Cancel")
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.close_editor(cx);
                        },
                    )),
                ),
        );

        let view = cx.entity();
        Some(popover(
            editor.anchor,
            px(320.0),
            palette,
            move |_window, cx| {
                view.update(cx, |this, cx| this.close_editor(cx));
            },
            card.into_any_element(),
        ))
    }
}

impl Render for KeymapManagementView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::from_theme(self.theme.read(cx));

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(18.0))
            .py(px(14.0))
            .border_b_1()
            .border_color(palette.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(16.0)).child("Keymap"))
                    .child(
                        div()
                            .mt(px(3.0))
                            .text_size(px(11.0))
                            .text_color(palette.muted)
                            .child("Browse every command and rebind its keys"),
                    ),
            )
            .child(
                button(
                    "keymap-edit-json",
                    palette,
                    ButtonVariant::Outline,
                    ButtonSize::Sm,
                )
                .child("Edit keymap.json")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.open_raw_document(window, cx);
                })),
            );

        let filter_row = div()
            .mx(px(16.0))
            .my(px(10.0))
            .h(px(30.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(palette.border)
            .child(IconName::Command.svg(palette.muted).size(px(12.0)))
            .child(
                div()
                    .flex_1()
                    .text_size(px(12.0))
                    .text_color(if self.filter.is_empty() {
                        palette.muted
                    } else {
                        palette.fg
                    })
                    .child(SharedString::from(if self.filter.is_empty() {
                        "Type to filter commands…".to_string()
                    } else {
                        self.filter.clone()
                    })),
            );

        let filters = segmented_control(
            "keymap-filter",
            palette,
            match self.row_filter {
                RowFilter::All => "all",
                RowFilter::Customized => "customized",
                RowFilter::Conflicts => "conflicts",
                RowFilter::Unbound => "unbound",
            },
        )
        .variant(SegmentVariant::Solid)
        .segments([
            ("all", "All"),
            ("customized", "Modified"),
            ("conflicts", "Conflicts"),
            ("unbound", "Unbound"),
        ])
        .on_select(cx.listener(|this, key: &SharedString, _window, cx| {
            this.row_filter = match key.as_ref() {
                "customized" => RowFilter::Customized,
                "conflicts" => RowFilter::Conflicts,
                "unbound" => RowFilter::Unbound,
                _ => RowFilter::All,
            };
            this.selected = 0;
            this.clamp_selection();
            cx.notify();
        }));

        let mut root = div()
            .track_focus(&self.focus)
            .key_context("Keymap")
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.bg)
            .text_color(palette.fg)
            .child(header);

        // Diagnostics banner from the loaded document.
        if let Some(snapshot) = &self.snapshot {
            if !snapshot.issues.is_empty() {
                let has_error = snapshot
                    .issues
                    .iter()
                    .any(|issue| matches!(issue.severity, file::Severity::Error));
                let lines = snapshot
                    .issues
                    .iter()
                    .take(4)
                    .map(|issue| format!("line {}: {}", issue.line, issue.message))
                    .collect::<Vec<_>>()
                    .join("  ·  ");
                let more = snapshot.issues.len().saturating_sub(4);
                let text = if more > 0 {
                    format!("{lines}  · +{more} more")
                } else {
                    lines
                };
                root = root.child(
                    banner(
                        if has_error {
                            Severity::Error
                        } else {
                            Severity::Warning
                        },
                        palette,
                    )
                    .child(SharedString::from(text))
                    .child(
                        button(
                            "keymap-open-diagnostics",
                            palette,
                            ButtonVariant::Ghost,
                            ButtonSize::Xs,
                        )
                        .child("Open keymap.json")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.open_raw_document(window, cx);
                            },
                        )),
                    ),
                );
            }
        }

        if self.external_change_pending {
            root = root.child(banner(Severity::Info, palette).child(SharedString::from(
                "keymap.json changed on disk — finish this edit to reload.",
            )));
        }

        root = root
            .child(filter_row)
            .child(div().mx(px(16.0)).mb(px(8.0)).child(filters));

        let body = if let Some(snapshot) = self.snapshot.clone() {
            let conflicts = snapshot.conflict_commands();
            let (visible, matched) = self.build_visible(&snapshot);
            if visible.is_empty() {
                div()
                    .flex()
                    .flex_1()
                    .justify_center()
                    .py(px(42.0))
                    .text_size(px(12.0))
                    .text_color(palette.muted)
                    .child(SharedString::from(empty_message(self.row_filter)))
                    .into_any_element()
            } else {
                let view = cx.entity();
                let selected = self.selected;
                let matched = std::rc::Rc::new(matched);
                let conflicts = std::rc::Rc::new(conflicts);
                let visible = std::rc::Rc::new(visible);
                let list_view = view.clone();
                let list = uniform_list(
                    "keymap-command-list",
                    visible.len(),
                    move |range, _window, _cx| {
                        range
                            .map(|i| match &visible[i] {
                                VisibleRow::Section {
                                    name,
                                    collapsed,
                                    count,
                                } => render_section_row(
                                    &list_view,
                                    name.clone(),
                                    *collapsed,
                                    *count,
                                    palette,
                                ),
                                VisibleRow::Command(index) => render_command_row(
                                    &list_view,
                                    &matched[*index],
                                    *index,
                                    selected == i,
                                    conflicts.contains(&matched[*index].command),
                                    palette,
                                ),
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .track_scroll(self.list_scroll.clone())
                .px(px(8.0))
                .pb(px(16.0))
                .flex_1();
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(list)
                    .into_any_element()
            }
        } else if self.load_error.is_none() {
            div()
                .flex()
                .flex_1()
                .justify_center()
                .py(px(42.0))
                .text_size(px(12.0))
                .text_color(palette.muted)
                .child("Loading keymap…")
                .into_any_element()
        } else {
            div().flex().flex_1().into_any_element()
        };

        root.child(body).children(self.render_editor(palette, cx))
    }
}

fn render_section_row(
    view: &Entity<KeymapManagementView>,
    name: String,
    collapsed: bool,
    count: usize,
    palette: Palette,
) -> AnyElement {
    let view = view.clone();
    let section = name.clone();
    div()
        .h(px(SECTION_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px(px(6.0))
        .child(
            disclosure(
                SharedString::from(format!("keymap-section-{name}")),
                name.clone(),
                collapsed,
                palette.muted,
                palette.fg,
            )
            .on_click(move |_: &ClickEvent, _window, cx| {
                view.update(cx, |this, cx| this.toggle_section(&section, cx));
            }),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child(SharedString::from(count.to_string())),
        )
        .into_any_element()
}

fn empty_message(filter: RowFilter) -> &'static str {
    match filter {
        RowFilter::All => "No commands match the filter.",
        RowFilter::Customized => "You haven't changed any keybindings yet.",
        RowFilter::Conflicts => "No binding conflicts.",
        RowFilter::Unbound => "Every command with a default is bound.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_view_type_is_ui_owned() {
        fn assert_focusable<T: Focusable>() {}
        assert_focusable::<KeymapManagementView>();
        let _ = labonair_command_palette_core::CommandId::OpenKeymapJson;
    }

    #[test]
    fn context_labels_round_trip() {
        for context in [
            CommandContext::Editor,
            CommandContext::Terminal,
            CommandContext::Sftp,
            CommandContext::Home,
            CommandContext::SshTerminal,
        ] {
            assert_eq!(
                context_from_label(runtime::context_name(context)),
                Some(context)
            );
            assert_eq!(
                context_from_label(runtime::context_label(context)),
                Some(context)
            );
        }
    }
}
