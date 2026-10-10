# Pinned Runtime Actions Without Default Key Bindings

**Status:** Cataloged from the pinned Linux runtime; user meaning, reachability, Labonair ownership, and behavior remain open
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`
**Runtime target:** Linux x86_64, Zed `1.20.0`, debug profile
**Evidence:** [`zed-actions-3569541-linux-x86_64.json`](evidence/zed-actions-3569541-linux-x86_64.json), SHA-256 `fda3ca179a9e6a27508646fde530a5ebf956252020d5505f45e2cfea60620db5`

## Inventory result

The pinned Linux runtime dump contains 1404 uniquely registered action names. After resolving the five default-keymap identifiers that the runtime declares as deprecated aliases, 591 registered actions are referenced by at least one of the macOS, Linux, or Windows default keymaps. The 813 rows below are registered by the Linux build but have no binding in any of those three default-map files. This does not mean that every action is unbound in alternate profiles: 312 of these actions appear in the enumerated macOS profile, Vim/Helix, or macOS-specific override layers in [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md), and 32 action spellings appear in the shared Linux/Windows profile or override layers in [`keymap-nonmacos-requirements.md`](keymap-nonmacos-requirements.md). Those sets overlap; do not add their counts. Their effects and platform dispatch remain open.

The inventory does not show that these actions are unreachable: a command palette, menu, pointer interaction, extension, or programmatic caller may invoke an action. Per-action user goals and discovery routes need independent classification. Linux runtime registration does not establish registration on macOS. `Pending` means there is no Labonair behavior or end-to-end invocation evidence recorded for that action.

The evidence JSON preserves each runtime action's exact name, human-readable label, optional documentation, JSON schema, and deprecated aliases, along with schema definitions. It was generated from the pinned checkout in
`zed-refrence/zed/` with:

```text
LIBRARY_PATH=/tmp/labonair-native-libs CARGO_TARGET_DIR=/tmp/labonair-pinned-zed-target ZED_REFERENCE_TARGET_DIR=/tmp/labonair-pinned-zed-target cargo run --locked -p zed -- --dump-all-actions
```

The temporary library directory links the host's installed
`libX11-xcb.so.1` under the linker name required by this build. No package was
installed and the pinned checkout was not modified. `Documentation` and
`Arguments` columns below indicate metadata presence; they are not acceptance
evidence. Requirement IDs use
`ACT-REGISTRY-LINUX-<namespace>-<action>` and are stable for this pinned inventory.

## Shared acceptance contract

For every row, independently establish the user's goal and full invocation flow; determine whether the action is user-facing, internal, or obsolete; record active and inactive contexts, pointer and keyboard routes, arguments, state changes, cancellation/error/recovery behavior, relevant settings and persistence, canonical Labonair owner, typed public route, dependencies, and linked runtime/test evidence. Mark non-applicable properties with evidence and a reason. A catalog entry alone does not satisfy these fields.

| Requirement ID | Registered action | Runtime label | Documentation | Arguments schema | Deprecated aliases | Semantic result and entry point | Canonical owner / Labonair route | End-to-end evidence |
|---|---|---|---|---|---|---|---|---|
| `ACT-REGISTRY-LINUX-action-Sequence` | `action::Sequence` | `action: sequence` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-activity_indicator-ShowErrorMessage` | `activity_indicator::ShowErrorMessage` | `activity indicator: show error message` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-AuthorizeToolCall` | `agent::AuthorizeToolCall` | `agent: authorize tool call` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-CopyThreadToClipboard` | `agent::CopyThreadToClipboard` | `agent: copy thread to clipboard` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-CreateSkillFromUrl` | `agent::CreateSkillFromUrl` | `agent: create skill from url` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-FocusAgent` | `agent::FocusAgent` | `agent: focus agent` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-FocusDown` | `agent::FocusDown` | `agent: focus down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-FocusLeft` | `agent::FocusLeft` | `agent: focus left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-FocusRight` | `agent::FocusRight` | `agent: focus right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-FocusUp` | `agent::FocusUp` | `agent: focus up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-Follow` | `agent::Follow` | `agent: follow` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ImportThreadsFromOtherChannels` | `agent::ImportThreadsFromOtherChannels` | `agent: import threads from other channels` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-LoadThreadFromClipboard` | `agent::LoadThreadFromClipboard` | `agent: load thread from clipboard` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-LogoutAgent` | `agent::LogoutAgent` | `agent: logout agent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-NewExternalAgentThread` | `agent::NewExternalAgentThread` | `agent: new external agent thread` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-NewNativeAgentThreadFromSummary` | `agent::NewNativeAgentThreadFromSummary` | `agent: new native agent thread from summary` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-NewTerminalThread` | `agent::NewTerminalThread` | `agent: new terminal thread` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-OpenActiveThreadAsMarkdown` | `agent::OpenActiveThreadAsMarkdown` | `agent: open active thread as markdown` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-OpenGlobalAGENTS.mdRules` | `agent::OpenGlobalAGENTS.mdRules` | `agent: open global AGENTS.md rules` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-OpenOnboardingModal` | `agent::OpenOnboardingModal` | `agent: open onboarding modal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-OpenProjectAGENTS.mdRules` | `agent::OpenProjectAGENTS.mdRules` | `agent: open project AGENTS.md rules` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-OpenSkillCreator` | `agent::OpenSkillCreator` | `agent: open skill creator` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ReauthenticateAgent` | `agent::ReauthenticateAgent` | `agent: reauthenticate agent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-RerunRulesToSkillsMigration` | `agent::RerunRulesToSkillsMigration` | `agent: rerun rules to skills migration` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ResetAgentZoom` | `agent::ResetAgentZoom` | `agent: reset agent zoom` | Present | None | None | With an AgentPanel in the current Workspace, clear the app-global transient Agent UI and Agent buffer font overrides; restore their configured fallbacks without changing Settings: [font-size increase and reset classification](action-classification-font-size-increase-reset.md) | AI (`labonair-ai`) owns the Agent action and panel guard; Themes (`labonair-theme`) owns typography reset state | No Labonair Agent panel or reset action. No default key binding or menu item; command-palette reachability is unverified | Pending (Linux catalog only; macOS registration and runtime interaction not observed) |
| `ACT-REGISTRY-LINUX-agent-ResetFastModeWarnings` | `agent::ResetFastModeWarnings` | `agent: reset fast mode warnings` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ResetOnboarding` | `agent::ResetOnboarding` | `agent: reset onboarding` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ResetTrialEndUpsell` | `agent::ResetTrialEndUpsell` | `agent: reset trial end upsell` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ResetTrialUpsell` | `agent::ResetTrialUpsell` | `agent: reset trial upsell` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ResolveConflictedFilesWithAgent` | `agent::ResolveConflictedFilesWithAgent` | `agent: resolve conflicted files with agent` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ResolveConflictsWithAgent` | `agent::ResolveConflictsWithAgent` | `agent: resolve conflicts with agent` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ReviewBranchDiff` | `agent::ReviewBranchDiff` | `agent: review branch diff` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-SelectAgent` | `agent::SelectAgent` | `agent: select agent` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-SelectPermissionGranularity` | `agent::SelectPermissionGranularity` | `agent: select permission granularity` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-Toggle` | `agent::Toggle` | `agent: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-agent-ToggleCommandPattern` | `agent::ToggleCommandPattern` | `agent: toggle command pattern` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-auto_update-Check` | `auto_update::Check` | `auto update: check` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-auto_update-DismissMessage` | `auto_update::DismissMessage` | `auto update: dismiss message` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-auto_update-ViewReleaseNotes` | `auto_update::ViewReleaseNotes` | `auto update: view release notes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-auto_update-ViewReleaseNotesLocally` | `auto_update::ViewReleaseNotesLocally` | `auto update: view release notes locally` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-bedrock-Tab` | `bedrock::Tab` | `bedrock: tab` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-bedrock-TabPrev` | `bedrock::TabPrev` | `bedrock: tab prev` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-branch_picker-ShowAllBranches` | `branch_picker::ShowAllBranches` | `branch picker: show all branches` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-branch_picker-ShowLocalBranches` | `branch_picker::ShowLocalBranches` | `branch picker: show local branches` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-branch_picker-ShowRemoteBranches` | `branch_picker::ShowRemoteBranches` | `branch picker: show remote branches` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-call_hierarchy-ShowOutgoingCalls` | `call_hierarchy::ShowOutgoingCalls` | `call hierarchy: show outgoing calls` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-channel_modal-RemoveMember` | `channel_modal::RemoveMember` | `channel modal: remove member` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-channel_modal-SelectNextControl` | `channel_modal::SelectNextControl` | `channel modal: select next control` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-channel_modal-ToggleMemberAdmin` | `channel_modal::ToggleMemberAdmin` | `channel modal: toggle member admin` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-cli-InstallCliBinary` | `cli::InstallCliBinary` | `cli: install cli binary` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-cli-RegisterZedScheme` | `cli::RegisterZedScheme` | `cli: register zed scheme` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-client-Reconnect` | `client::Reconnect` | `client: reconnect` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-client-SignIn` | `client::SignIn` | `client: sign in` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-client-SignOut` | `client::SignOut` | `client: sign out` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-CopyLink` | `collab::CopyLink` | `collab: copy link` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-CopyRoomId` | `collab::CopyRoomId` | `collab: copy room id` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-Deafen` | `collab::Deafen` | `collab: deafen` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-LeaveCall` | `collab::LeaveCall` | `collab: leave call` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-Mute` | `collab::Mute` | `collab: mute` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-OpenChannelNotes` | `collab::OpenChannelNotes` | `collab: open channel notes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-OpenChannelNotesById` | `collab::OpenChannelNotesById` | `collab: open channel notes by id` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-ScreenShare` | `collab::ScreenShare` | `collab: screen share` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-ShareProject` | `collab::ShareProject` | `collab: share project` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-ShowCallStats` | `collab::ShowCallStats` | `collab: show call stats` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-SimulateUpdateAvailable` | `collab::SimulateUpdateAvailable` | `collab: simulate update available` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-SwitchBranch` | `collab::SwitchBranch` | `collab: switch branch` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-ToggleProjectMenu` | `collab::ToggleProjectMenu` | `collab: toggle project menu` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab-ToggleUserMenu` | `collab::ToggleUserMenu` | `collab: toggle user menu` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab_panel-CollapseSelectedChannel` | `collab_panel::CollapseSelectedChannel` | `collab panel: collapse selected channel` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab_panel-ExpandSelectedChannel` | `collab_panel::ExpandSelectedChannel` | `collab panel: expand selected channel` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab_panel-MoveSelected` | `collab_panel::MoveSelected` | `collab panel: move selected` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab_panel-Secondary` | `collab_panel::Secondary` | `collab panel: secondary` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab_panel-StartMoveChannel` | `collab_panel::StartMoveChannel` | `collab panel: start move channel` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-collab_panel-Toggle` | `collab_panel::Toggle` | `collab panel: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-context_server-Restart` | `context_server::Restart` | `context server: restart` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-copilot_edit_predictions-Reinstall` | `copilot_edit_predictions::Reinstall` | `copilot edit predictions: reinstall` | Present | None | Present | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debug_panel-Toggle` | `debug_panel::Toggle` | `debug panel: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-ClearAllBreakpoints` | `debugger::ClearAllBreakpoints` | `debugger: clear all breakpoints` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-ContinueThread` | `debugger::ContinueThread` | `debugger: continue thread` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-Detach` | `debugger::Detach` | `debugger: detach` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-EvaluateSelectedText` | `debugger::EvaluateSelectedText` | `debugger: evaluate selected text` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusBreakpointList` | `debugger::FocusBreakpointList` | `debugger: focus breakpoint list` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusConsole` | `debugger::FocusConsole` | `debugger: focus console` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusFrames` | `debugger::FocusFrames` | `debugger: focus frames` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusLoadedSources` | `debugger::FocusLoadedSources` | `debugger: focus loaded sources` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusModules` | `debugger::FocusModules` | `debugger: focus modules` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusTerminal` | `debugger::FocusTerminal` | `debugger: focus terminal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-FocusVariables` | `debugger::FocusVariables` | `debugger: focus variables` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-GoToSelectedAddress` | `debugger::GoToSelectedAddress` | `debugger: go to selected address` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-OpenProjectDebugTasks` | `debugger::OpenProjectDebugTasks` | `debugger: open project debug tasks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-Restart` | `debugger::Restart` | `debugger: restart` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-RunToCursor` | `debugger::RunToCursor` | `debugger: run to cursor` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-StepBack` | `debugger::StepBack` | `debugger: step back` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-ToggleDataBreakpoint` | `debugger::ToggleDataBreakpoint` | `debugger: toggle data breakpoint` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-ToggleIgnoreBreakpoints` | `debugger::ToggleIgnoreBreakpoints` | `debugger: toggle ignore breakpoints` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-debugger-ToggleUserFrames` | `debugger::ToggleUserFrames` | `debugger: toggle user frames` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-CopyAccessibilityTree` | `dev::CopyAccessibilityTree` | `dev: copy accessibility tree` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-CopyDebugAdapterArguments` | `dev::CopyDebugAdapterArguments` | `dev: copy debug adapter arguments` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-DumpAccessibilityTree` | `dev::DumpAccessibilityTree` | `dev: dump accessibility tree` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-DumpInputLatencyHistogram` | `dev::DumpInputLatencyHistogram` | `dev: dump input latency histogram` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-DumpWorkspaceInfo` | `dev::DumpWorkspaceInfo` | `dev: dump workspace info` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-HangAction` | `dev::HangAction` | `dev: hang action` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-HangBackground` | `dev::HangBackground` | `dev: hang background` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-HangForeground` | `dev::HangForeground` | `dev: hang foreground` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenAcpLogs` | `dev::OpenAcpLogs` | `dev: open acp logs` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenDebugAdapterLogs` | `dev::OpenDebugAdapterLogs` | `dev: open debug adapter logs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenEditPredictionContextView` | `dev::OpenEditPredictionContextView` | `dev: open edit prediction context view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenHighlightsTreeView` | `dev::OpenHighlightsTreeView` | `dev: open highlights tree view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenKeyContextView` | `dev::OpenKeyContextView` | `dev: open key context view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenLanguageServerLogs` | `dev::OpenLanguageServerLogs` | `dev: open language server logs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenSyntaxTreeView` | `dev::OpenSyntaxTreeView` | `dev: open syntax tree view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenThemePreview` | `dev::OpenThemePreview` | `dev: open theme preview` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-OpenUrlPrompt` | `dev::OpenUrlPrompt` | `dev: open url prompt` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-ShowAllSidebarThreadMetadata` | `dev::ShowAllSidebarThreadMetadata` | `dev: show all sidebar thread metadata` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-ShowGitJobQueue` | `dev::ShowGitJobQueue` | `dev: show git job queue` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-ShowThreadMetadata` | `dev::ShowThreadMetadata` | `dev: show thread metadata` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-dev-ShowWorkspaceError` | `dev::ShowWorkspaceError` | `dev: show workspace error` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-diagnostics-DeployCurrentFile` | `diagnostics::DeployCurrentFile` | `diagnostics: deploy current file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-diagnostics-ToggleWarnings` | `diagnostics::ToggleWarnings` | `diagnostics: toggle warnings` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-edit_prediction-ClearHistory` | `edit_prediction::ClearHistory` | `edit prediction: clear history` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-edit_prediction-ResetOnboarding` | `edit_prediction::ResetOnboarding` | `edit prediction: reset onboarding` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-AlignSelections` | `editor::AlignSelections` | `editor: align selections` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ApplyAllDiffHunks` | `editor::ApplyAllDiffHunks` | `editor: apply all diff hunks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ApplyDiffHunk` | `editor::ApplyDiffHunk` | `editor: apply diff hunk` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-AutoIndent` | `editor::AutoIndent` | `editor: auto indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-BlamePreviousRevision` | `editor::BlamePreviousRevision` | `editor: blame previous revision` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-BlameRevision` | `editor::BlameRevision` | `editor: blame revision` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CancelEditReviewComment` | `editor::CancelEditReviewComment` | `editor: cancel edit review comment` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CancelFlycheck` | `editor::CancelFlycheck` | `editor: cancel flycheck` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CancelLanguageServerWork` | `editor::CancelLanguageServerWork` | `editor: cancel language server work` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ClearFlycheck` | `editor::ClearFlycheck` | `editor: clear flycheck` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CollapseAllDiffHunks` | `editor::CollapseAllDiffHunks` | `editor: collapse all diff hunks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConfirmCompletionInsert` | `editor::ConfirmCompletionInsert` | `editor: confirm completion insert` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConfirmEditReviewComment` | `editor::ConfirmEditReviewComment` | `editor: confirm edit review comment` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertFromBase64` | `editor::ConvertFromBase64` | `editor: convert from base64` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertIndentationToSpaces` | `editor::ConvertIndentationToSpaces` | `editor: convert indentation to spaces` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertIndentationToTabs` | `editor::ConvertIndentationToTabs` | `editor: convert indentation to tabs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToBase64` | `editor::ConvertToBase64` | `editor: convert to base64` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToKebabCase` | `editor::ConvertToKebabCase` | `editor: convert to kebab case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToLowerCamelCase` | `editor::ConvertToLowerCamelCase` | `editor: convert to lower camel case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToLowerCase` | `editor::ConvertToLowerCase` | `editor: convert to lower case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToOppositeCase` | `editor::ConvertToOppositeCase` | `editor: convert to opposite case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToRot13` | `editor::ConvertToRot13` | `editor: convert to rot13` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToRot47` | `editor::ConvertToRot47` | `editor: convert to rot47` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToSentenceCase` | `editor::ConvertToSentenceCase` | `editor: convert to sentence case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToSnakeCase` | `editor::ConvertToSnakeCase` | `editor: convert to snake case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToTitleCase` | `editor::ConvertToTitleCase` | `editor: convert to title case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToUpperCamelCase` | `editor::ConvertToUpperCamelCase` | `editor: convert to upper camel case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ConvertToUpperCase` | `editor::ConvertToUpperCase` | `editor: convert to upper case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CopyAndTrim` | `editor::CopyAndTrim` | `editor: copy and trim` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CopyFileLocation` | `editor::CopyFileLocation` | `editor: copy file location` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CopyFileName` | `editor::CopyFileName` | `editor: copy file name` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CopyFileNameWithoutExtension` | `editor::CopyFileNameWithoutExtension` | `editor: copy file name without extension` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CopyHighlightJson` | `editor::CopyHighlightJson` | `editor: copy highlight json` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CopyPermalinkToLine` | `editor::CopyPermalinkToLine` | `editor: copy permalink to line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-CutToEndOfLine` | `editor::CutToEndOfLine` | `editor: cut to end of line` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-DeleteReviewComment` | `editor::DeleteReviewComment` | `editor: delete review comment` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-DiffClipboardWithSelection` | `editor::DiffClipboardWithSelection` | `editor: diff clipboard with selection` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-DisableBreakpoint` | `editor::DisableBreakpoint` | `editor: disable breakpoint` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-DuplicateSelection` | `editor::DuplicateSelection` | `editor: duplicate selection` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-EditBookmark` | `editor::EditBookmark` | `editor: edit bookmark` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-EditReviewComment` | `editor::EditReviewComment` | `editor: edit review comment` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-EnableBreakpoint` | `editor::EnableBreakpoint` | `editor: enable breakpoint` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ExpandExcerptsDown` | `editor::ExpandExcerptsDown` | `editor: expand excerpts down` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ExpandExcerptsUp` | `editor::ExpandExcerptsUp` | `editor: expand excerpts up` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ExpandMacroRecursively` | `editor::ExpandMacroRecursively` | `editor: expand macro recursively` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-FindNextMatch` | `editor::FindNextMatch` | `editor: find next match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-FindPreviousMatch` | `editor::FindPreviousMatch` | `editor: find previous match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-FoldAtLevel` | `editor::FoldAtLevel` | `editor: fold at level` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-FoldFunctionBodies` | `editor::FoldFunctionBodies` | `editor: fold function bodies` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-FoldSelectedRanges` | `editor::FoldSelectedRanges` | `editor: fold selected ranges` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToImplementationSplit` | `editor::GoToImplementationSplit` | `editor: go to implementation split` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToNextBookmark` | `editor::GoToNextBookmark` | `editor: go to next bookmark` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToNextDocumentHighlight` | `editor::GoToNextDocumentHighlight` | `editor: go to next document highlight` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToNextReference` | `editor::GoToNextReference` | `editor: go to next reference` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToNextSymbol` | `editor::GoToNextSymbol` | `editor: go to next symbol` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToParentModule` | `editor::GoToParentModule` | `editor: go to parent module` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToPreviousBookmark` | `editor::GoToPreviousBookmark` | `editor: go to previous bookmark` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToPreviousDocumentHighlight` | `editor::GoToPreviousDocumentHighlight` | `editor: go to previous document highlight` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToPreviousReference` | `editor::GoToPreviousReference` | `editor: go to previous reference` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-GoToPreviousSymbol` | `editor::GoToPreviousSymbol` | `editor: go to previous symbol` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-HalfPageDown` | `editor::HalfPageDown` | `editor: half page down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-HalfPageUp` | `editor::HalfPageUp` | `editor: half page up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-HandleInput` | `editor::HandleInput` | `editor: handle input` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-InsertSnippet` | `editor::InsertSnippet` | `editor: insert snippet` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-InsertUuidV4` | `editor::InsertUuidV4` | `editor: insert uuid v4` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-InsertUuidV7` | `editor::InsertUuidV7` | `editor: insert uuid v7` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveDownByLines` | `editor::MoveDownByLines` | `editor: move down by lines` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveToEndOfExcerpt` | `editor::MoveToEndOfExcerpt` | `editor: move to end of excerpt` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveToEndOfLargerSyntaxNode` | `editor::MoveToEndOfLargerSyntaxNode` | `editor: move to end of larger syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveToEndOfPreviousExcerpt` | `editor::MoveToEndOfPreviousExcerpt` | `editor: move to end of previous excerpt` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveToNextCommentParagraph` | `editor::MoveToNextCommentParagraph` | `editor: move to next comment paragraph` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveToPreviousCommentParagraph` | `editor::MoveToPreviousCommentParagraph` | `editor: move to previous comment paragraph` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveToStartOfLargerSyntaxNode` | `editor::MoveToStartOfLargerSyntaxNode` | `editor: move to start of larger syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-MoveUpByLines` | `editor::MoveUpByLines` | `editor: move up by lines` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-NextScreen` | `editor::NextScreen` | `editor: next screen` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-OpenDocs` | `editor::OpenDocs` | `editor: open docs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-OpenGitBlameCommit` | `editor::OpenGitBlameCommit` | `editor: open git blame commit` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-OpenPermalinkToLine` | `editor::OpenPermalinkToLine` | `editor: open permalink to line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-OpenProposedChangesEditor` | `editor::OpenProposedChangesEditor` | `editor: open proposed changes editor` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-OpenSelectedFilename` | `editor::OpenSelectedFilename` | `editor: open selected filename` | Present | None | Present | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-OpenUrl` | `editor::OpenUrl` | `editor: open url` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ReloadFile` | `editor::ReloadFile` | `editor: reload file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-RestartLanguageServer` | `editor::RestartLanguageServer` | `editor: restart language server` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ReverseLines` | `editor::ReverseLines` | `editor: reverse lines` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-RotateSelectionsBackward` | `editor::RotateSelectionsBackward` | `editor: rotate selections backward` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-RotateSelectionsForward` | `editor::RotateSelectionsForward` | `editor: rotate selections forward` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-RunFlycheck` | `editor::RunFlycheck` | `editor: run flycheck` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SaveLocation` | `editor::SaveLocation` | `editor: save location` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ScrollCursorBottom` | `editor::ScrollCursorBottom` | `editor: scroll cursor bottom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ScrollCursorCenterTopBottom` | `editor::ScrollCursorCenterTopBottom` | `editor: scroll cursor center top bottom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ScrollCursorTop` | `editor::ScrollCursorTop` | `editor: scroll cursor top` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectAroundDelimiters` | `editor::SelectAroundDelimiters` | `editor: select around delimiters` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectDownByLines` | `editor::SelectDownByLines` | `editor: select down by lines` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectInsideDelimiters` | `editor::SelectInsideDelimiters` | `editor: select inside delimiters` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectToEndOfExcerpt` | `editor::SelectToEndOfExcerpt` | `editor: select to end of excerpt` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectToEndOfLargerSyntaxNode` | `editor::SelectToEndOfLargerSyntaxNode` | `editor: select to end of larger syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectToEndOfPreviousExcerpt` | `editor::SelectToEndOfPreviousExcerpt` | `editor: select to end of previous excerpt` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectToStartOfLargerSyntaxNode` | `editor::SelectToStartOfLargerSyntaxNode` | `editor: select to start of larger syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SelectUpByLines` | `editor::SelectUpByLines` | `editor: select up by lines` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SendReviewToAgent` | `editor::SendReviewToAgent` | `editor: send review to agent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SetMark` | `editor::SetMark` | `editor: set mark` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ShuffleLines` | `editor::ShuffleLines` | `editor: shuffle lines` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SortLinesByLength` | `editor::SortLinesByLength` | `editor: sort lines by length` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SortLinesCaseInsensitive` | `editor::SortLinesCaseInsensitive` | `editor: sort lines case insensitive` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SortLinesCaseSensitive` | `editor::SortLinesCaseSensitive` | `editor: sort lines case sensitive` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SpawnNearestTask` | `editor::SpawnNearestTask` | `editor: spawn nearest task` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SplitSelectionIntoLines` | `editor::SplitSelectionIntoLines` | `editor: split selection into lines` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-StopLanguageServer` | `editor::StopLanguageServer` | `editor: stop language server` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SubmitDiffReviewComment` | `editor::SubmitDiffReviewComment` | `editor: submit diff review comment` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SwapSelectionEnds` | `editor::SwapSelectionEnds` | `editor: swap selection ends` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-SwitchSourceHeader` | `editor::SwitchSourceHeader` | `editor: switch source header` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleAllDiffHunks` | `editor::ToggleAllDiffHunks` | `editor: toggle all diff hunks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleAutoSignatureHelp` | `editor::ToggleAutoSignatureHelp` | `editor: toggle auto signature help` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleBookmark` | `editor::ToggleBookmark` | `editor: toggle bookmark` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleBookmarkWithLabel` | `editor::ToggleBookmarkWithLabel` | `editor: toggle bookmark with label` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleBreadcrumb` | `editor::ToggleBreadcrumb` | `editor: toggle breadcrumb` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleCase` | `editor::ToggleCase` | `editor: toggle case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleCodeLens` | `editor::ToggleCodeLens` | `editor: toggle code lens` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleDiagnostics` | `editor::ToggleDiagnostics` | `editor: toggle diagnostics` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleFocus` | `editor::ToggleFocus` | `editor: toggle focus` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleFoldRecursive` | `editor::ToggleFoldRecursive` | `editor: toggle fold recursive` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleGitBlameInline` | `editor::ToggleGitBlameInline` | `editor: toggle git blame inline` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleIndentGuides` | `editor::ToggleIndentGuides` | `editor: toggle indent guides` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleInlineDiagnostics` | `editor::ToggleInlineDiagnostics` | `editor: toggle inline diagnostics` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleInlineValues` | `editor::ToggleInlineValues` | `editor: toggle inline values` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleMinimap` | `editor::ToggleMinimap` | `editor: toggle minimap` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleRelativeLineNumbers` | `editor::ToggleRelativeLineNumbers` | `editor: toggle relative line numbers` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleReviewCommentsExpanded` | `editor::ToggleReviewCommentsExpanded` | `editor: toggle review comments expanded` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleSelectionMenu` | `editor::ToggleSelectionMenu` | `editor: toggle selection menu` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleSemanticHighlights` | `editor::ToggleSemanticHighlights` | `editor: toggle semantic highlights` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleSplitDiff` | `editor::ToggleSplitDiff` | `editor: toggle split diff` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ToggleTabBar` | `editor::ToggleTabBar` | `editor: toggle tab bar` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-UniqueLinesCaseInsensitive` | `editor::UniqueLinesCaseInsensitive` | `editor: unique lines case insensitive` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-UniqueLinesCaseSensitive` | `editor::UniqueLinesCaseSensitive` | `editor: unique lines case sensitive` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-UnwrapSyntaxNode` | `editor::UnwrapSyntaxNode` | `editor: unwrap syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-ViewBookmarks` | `editor::ViewBookmarks` | `editor: view bookmarks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-WrapSelectionsInTag` | `editor::WrapSelectionsInTag` | `editor: wrap selections in tag` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-editor-WrapWithAbbreviation` | `editor::WrapWithAbbreviation` | `editor: wrap with abbreviation` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-feedback-EmailZed` | `feedback::EmailZed` | `feedback: email zed` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-feedback-FileBugReport` | `feedback::FileBugReport` | `feedback: file bug report` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-feedback-RequestFeature` | `feedback::RequestFeature` | `feedback: request feature` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-file_finder-OpenWithoutDismiss` | `file_finder::OpenWithoutDismiss` | `file finder: open without dismiss` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-file_finder-SelectPrevious` | `file_finder::SelectPrevious` | `file finder: select previous` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-Add` | `git::Add` | `git: add` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-AddToGitInfoExclude` | `git::AddToGitInfoExclude` | `git: add to git info exclude` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-AddToGitignore` | `git::AddToGitignore` | `git: add to gitignore` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CheckoutBranch` | `git::CheckoutBranch` | `git: checkout branch` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-Clone` | `git::Clone` | `git: clone` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CompareWithBranch` | `git::CompareWithBranch` | `git: compare with branch` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CopyBranchName` | `git::CopyBranchName` | `git: copy branch name` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CopyFilePermalink` | `git::CopyFilePermalink` | `git: copy file permalink` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CreatePullRequest` | `git::CreatePullRequest` | `git: create pull request` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CreateRemote` | `git::CreateRemote` | `git: create remote` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-CreateWorktree` | `git::CreateWorktree` | `git: create worktree` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-DiffBranch` | `git::DiffBranch` | `git: diff branch` | Present | None | Present | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-DiffHead` | `git::DiffHead` | `git: diff head` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-FetchFrom` | `git::FetchFrom` | `git: fetch from` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-FileHistory` | `git::FileHistory` | `git: file history` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-FilterRemotes` | `git::FilterRemotes` | `git: filter remotes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-Init` | `git::Init` | `git: init` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-LeaderAndFollower` | `git::LeaderAndFollower` | `git: leader and follower` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-OpenFileAtHead` | `git::OpenFileAtHead` | `git: open file at head` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-OpenFileDiff` | `git::OpenFileDiff` | `git: open file diff` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-OpenFilePermalink` | `git::OpenFilePermalink` | `git: open file permalink` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-OpenWorktreeInNewWindow` | `git::OpenWorktreeInNewWindow` | `git: open worktree in new window` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-PushTo` | `git::PushTo` | `git: push to` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-RenameBranch` | `git::RenameBranch` | `git: rename branch` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-SelectRepo` | `git::SelectRepo` | `git: select repo` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-Signoff` | `git::Signoff` | `git: signoff` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-SkipHooks` | `git::SkipHooks` | `git: skip hooks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-StageSection` | `git::StageSection` | `git: stage section` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-StashAll` | `git::StashAll` | `git: stash all` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-StashApply` | `git::StashApply` | `git: stash apply` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-StashPop` | `git::StashPop` | `git: stash pop` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-StashStaged` | `git::StashStaged` | `git: stash staged` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-StashTracked` | `git::StashTracked` | `git: stash tracked` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-Switch` | `git::Switch` | `git: switch` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-SwitchWorktree` | `git::SwitchWorktree` | `git: switch worktree` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ToggleDiffBase` | `git::ToggleDiffBase` | `git: toggle diff base` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-Uncommit` | `git::Uncommit` | `git: uncommit` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-UnstageSection` | `git::UnstageSection` | `git: unstage section` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ViewCommit` | `git::ViewCommit` | `git: view commit` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ViewFile` | `git::ViewFile` | `git: view file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ViewStagedChanges` | `git::ViewStagedChanges` | `git: view staged changes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ViewStash` | `git::ViewStash` | `git: view stash` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ViewUncommittedChanges` | `git::ViewUncommittedChanges` | `git: view uncommitted changes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git-ViewUnstagedChanges` | `git::ViewUnstagedChanges` | `git: view unstaged changes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-CopyCommitSha` | `git_graph::CopyCommitSha` | `git graph: copy commit sha` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-CopyCommitTag` | `git_graph::CopyCommitTag` | `git graph: copy commit tag` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-FocusSearch` | `git_graph::FocusSearch` | `git graph: focus search` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-Open` | `git_graph::Open` | `git graph: open` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-OpenAtCommit` | `git_graph::OpenAtCommit` | `git graph: open at commit` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-OpenCommitView` | `git_graph::OpenCommitView` | `git graph: open commit view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-ScrollDown` | `git_graph::ScrollDown` | `git graph: scroll down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-ScrollUp` | `git_graph::ScrollUp` | `git graph: scroll up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_graph-ToggleChangedFilesView` | `git_graph::ToggleChangedFilesView` | `git graph: toggle changed files view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_onboarding-OpenGitIntegrationOnboarding` | `git_onboarding::OpenGitIntegrationOnboarding` | `git onboarding: open git integration onboarding` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-Close` | `git_panel::Close` | `git panel: close` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-OpenMenu` | `git_panel::OpenMenu` | `git panel: open menu` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-SetGroupByNone` | `git_panel::SetGroupByNone` | `git panel: set group by none` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-SetGroupByStaging` | `git_panel::SetGroupByStaging` | `git panel: set group by staging` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-SetGroupByStatus` | `git_panel::SetGroupByStatus` | `git panel: set group by status` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-SetSortByName` | `git_panel::SetSortByName` | `git panel: set sort by name` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-SetSortByPath` | `git_panel::SetSortByPath` | `git panel: set sort by path` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-Toggle` | `git_panel::Toggle` | `git panel: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-ToggleFillCoAuthors` | `git_panel::ToggleFillCoAuthors` | `git panel: toggle fill co authors` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-ToggleTreeView` | `git_panel::ToggleTreeView` | `git panel: toggle tree view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-ViewStagedChanges` | `git_panel::ViewStagedChanges` | `git panel: view staged changes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-git_panel-ViewUnstagedChanges` | `git_panel::ViewUnstagedChanges` | `git panel: view unstaged changes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-highlights_tree_view-ToggleSemanticTokens` | `highlights_tree_view::ToggleSemanticTokens` | `highlights tree view: toggle semantic tokens` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-highlights_tree_view-ToggleSyntaxTokens` | `highlights_tree_view::ToggleSyntaxTokens` | `highlights tree view: toggle syntax tokens` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-highlights_tree_view-ToggleTextHighlights` | `highlights_tree_view::ToggleTextHighlights` | `highlights tree view: toggle text highlights` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-icon_theme_selector-Toggle` | `icon_theme_selector::Toggle` | `icon theme selector: toggle` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-journal-NewJournalEntry` | `journal::NewJournalEntry` | `journal: new journal entry` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-keymap_editor-DeleteBinding` | `keymap_editor::DeleteBinding` | `keymap editor: delete binding` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-keymap_editor-ToggleExactKeystrokeMatching` | `keymap_editor::ToggleExactKeystrokeMatching` | `keymap editor: toggle exact keystroke matching` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-keymap_editor-ToggleNoActionBindings` | `keymap_editor::ToggleNoActionBindings` | `keymap editor: toggle no action bindings` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-line_ending_selector-Toggle` | `line_ending_selector::Toggle` | `line ending selector: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-markdown-OpenFollowingPreview` | `markdown::OpenFollowingPreview` | `markdown: open following preview` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-markdown-ToggleBlockQuote` | `markdown::ToggleBlockQuote` | `markdown: toggle block quote` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-menu-EndSlot` | `menu::EndSlot` | `menu: end slot` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-CloseWorkspaceSidebar` | `multi_workspace::CloseWorkspaceSidebar` | `multi workspace: close workspace sidebar` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-MoveProjectDown` | `multi_workspace::MoveProjectDown` | `multi workspace: move project down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-MoveProjectToNewWindow` | `multi_workspace::MoveProjectToNewWindow` | `multi workspace: move project to new window` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-MoveProjectUp` | `multi_workspace::MoveProjectUp` | `multi workspace: move project up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-NewThread` | `multi_workspace::NewThread` | `multi workspace: new thread` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-NextProject` | `multi_workspace::NextProject` | `multi workspace: next project` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-NextThread` | `multi_workspace::NextThread` | `multi workspace: next thread` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-PreviousProject` | `multi_workspace::PreviousProject` | `multi workspace: previous project` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-multi_workspace-PreviousThread` | `multi_workspace::PreviousThread` | `multi workspace: previous thread` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-notebook-ClearOutputs` | `notebook::ClearOutputs` | `notebook: clear outputs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-notebook-NotebookMoveDown` | `notebook::NotebookMoveDown` | `notebook: notebook move down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-notebook-NotebookMoveUp` | `notebook::NotebookMoveUp` | `notebook: notebook move up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-notebook-OpenNotebook` | `notebook::OpenNotebook` | `notebook: open notebook` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-onboarding-ResetHints` | `onboarding::ResetHints` | `onboarding: reset hints` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-CollapseAllEntries` | `outline_panel::CollapseAllEntries` | `outline panel: collapse all entries` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ExpandAllEntries` | `outline_panel::ExpandAllEntries` | `outline panel: expand all entries` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-FoldDirectory` | `outline_panel::FoldDirectory` | `outline panel: fold directory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ScrollCursorBottom` | `outline_panel::ScrollCursorBottom` | `outline panel: scroll cursor bottom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ScrollCursorCenter` | `outline_panel::ScrollCursorCenter` | `outline panel: scroll cursor center` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ScrollCursorTop` | `outline_panel::ScrollCursorTop` | `outline panel: scroll cursor top` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ScrollDown` | `outline_panel::ScrollDown` | `outline panel: scroll down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ScrollUp` | `outline_panel::ScrollUp` | `outline panel: scroll up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-SelectParent` | `outline_panel::SelectParent` | `outline panel: select parent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-Toggle` | `outline_panel::Toggle` | `outline panel: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ToggleActiveEditorPin` | `outline_panel::ToggleActiveEditorPin` | `outline panel: toggle active editor pin` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-ToggleSymbols` | `outline_panel::ToggleSymbols` | `outline panel: toggle symbols` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-outline_panel-UnfoldDirectory` | `outline_panel::UnfoldDirectory` | `outline panel: unfold directory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-AlternateFile` | `pane::AlternateFile` | `pane: alternate file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-CloseMultibufferItems` | `pane::CloseMultibufferItems` | `pane: close multibuffer items` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-GoToNewerTag` | `pane::GoToNewerTag` | `pane: go to newer tag` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-GoToOlderTag` | `pane::GoToOlderTag` | `pane: go to older tag` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-JoinAll` | `pane::JoinAll` | `pane: join all` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-JoinIntoNext` | `pane::JoinIntoNext` | `pane: join into next` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-SplitAndMoveDown` | `pane::SplitAndMoveDown` | `pane: split and move down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-SplitAndMoveLeft` | `pane::SplitAndMoveLeft` | `pane: split and move left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-SplitAndMoveRight` | `pane::SplitAndMoveRight` | `pane: split and move right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-SplitAndMoveUp` | `pane::SplitAndMoveUp` | `pane: split and move up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-SplitHorizontal` | `pane::SplitHorizontal` | `pane: split horizontal` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-SplitVertical` | `pane::SplitVertical` | `pane: split vertical` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-TogglePreviewTab` | `pane::TogglePreviewTab` | `pane: toggle preview tab` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-pane-UnpinAllTabs` | `pane::UnpinAllTabs` | `pane: unpin all tabs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-MultiSelectNext` | `picker::MultiSelectNext` | `picker: multi select next` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-SetPreviewBelow` | `picker::SetPreviewBelow` | `picker: set preview below` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-SetPreviewHidden` | `picker::SetPreviewHidden` | `picker: set preview hidden` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-SetPreviewRight` | `picker::SetPreviewRight` | `picker: set preview right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-ToggleActionsMenu` | `picker::ToggleActionsMenu` | `picker: toggle actions menu` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-ToggleMultiSelect` | `picker::ToggleMultiSelect` | `picker: toggle multi select` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-picker-TogglePreview` | `picker::TogglePreview` | `picker: toggle preview` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-CollapseSelectedEntryAndChildren` | `project_panel::CollapseSelectedEntryAndChildren` | `project panel: collapse selected entry and children` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-DownloadFromRemote` | `project_panel::DownloadFromRemote` | `project panel: download from remote` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ExpandSelectedEntryAndChildren` | `project_panel::ExpandSelectedEntryAndChildren` | `project panel: expand selected entry and children` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-FoldDirectory` | `project_panel::FoldDirectory` | `project panel: fold directory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-OpenMarkdownPreview` | `project_panel::OpenMarkdownPreview` | `project panel: open markdown preview` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-OpenPermanent` | `project_panel::OpenPermanent` | `project panel: open permanent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-OpenSplitHorizontal` | `project_panel::OpenSplitHorizontal` | `project panel: open split horizontal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-OpenSplitVertical` | `project_panel::OpenSplitVertical` | `project panel: open split vertical` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-RemoveFromProject` | `project_panel::RemoveFromProject` | `project panel: remove from project` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ScrollCursorBottom` | `project_panel::ScrollCursorBottom` | `project panel: scroll cursor bottom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ScrollCursorCenter` | `project_panel::ScrollCursorCenter` | `project panel: scroll cursor center` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ScrollCursorTop` | `project_panel::ScrollCursorTop` | `project panel: scroll cursor top` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ScrollDown` | `project_panel::ScrollDown` | `project panel: scroll down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ScrollUp` | `project_panel::ScrollUp` | `project panel: scroll up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectNextDiagnostic` | `project_panel::SelectNextDiagnostic` | `project panel: select next diagnostic` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectNextDirectory` | `project_panel::SelectNextDirectory` | `project panel: select next directory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectNextGitEntry` | `project_panel::SelectNextGitEntry` | `project panel: select next git entry` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectParent` | `project_panel::SelectParent` | `project panel: select parent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectPrevDiagnostic` | `project_panel::SelectPrevDiagnostic` | `project panel: select prev diagnostic` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectPrevDirectory` | `project_panel::SelectPrevDirectory` | `project panel: select prev directory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-SelectPrevGitEntry` | `project_panel::SelectPrevGitEntry` | `project panel: select prev git entry` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-Toggle` | `project_panel::Toggle` | `project panel: toggle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ToggleHideGitIgnore` | `project_panel::ToggleHideGitIgnore` | `project panel: toggle hide git ignore` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-ToggleHideHidden` | `project_panel::ToggleHideHidden` | `project panel: toggle hide hidden` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_panel-UnfoldDirectory` | `project_panel::UnfoldDirectory` | `project panel: unfold directory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-project_search-NextField` | `project_search::NextField` | `project search: next field` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-projects-InitializeDevContainer` | `projects::InitializeDevContainer` | `projects: initialize dev container` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-projects-OpenDevContainer` | `projects::OpenDevContainer` | `projects: open dev container` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-projects-OpenFolderInWsl` | `projects::OpenFolderInWsl` | `projects: open folder in wsl` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-projects-OpenWsl` | `projects::OpenWsl` | `projects: open wsl` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-remote_debug-SimulateDisconnect` | `remote_debug::SimulateDisconnect` | `remote debug: simulate disconnect` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-remote_debug-SimulateTimeout` | `remote_debug::SimulateTimeout` | `remote debug: simulate timeout` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-remote_debug-SimulateTimeoutExhausted` | `remote_debug::SimulateTimeoutExhausted` | `remote debug: simulate timeout exhausted` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-ClearCurrentOutput` | `repl::ClearCurrentOutput` | `repl: clear current output` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-ClearOutputs` | `repl::ClearOutputs` | `repl: clear outputs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-Interrupt` | `repl::Interrupt` | `repl: interrupt` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-RefreshKernelspecs` | `repl::RefreshKernelspecs` | `repl: refresh kernelspecs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-Restart` | `repl::Restart` | `repl: restart` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-Sessions` | `repl::Sessions` | `repl: sessions` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-repl-Shutdown` | `repl::Shutdown` | `repl: shutdown` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-search-CycleMode` | `search::CycleMode` | `search: cycle mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-skill_creator-Cancel` | `skill_creator::Cancel` | `skill creator: cancel` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-snippets-ConfigureSnippets` | `snippets::ConfigureSnippets` | `snippets: configure snippets` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-snippets-OpenFolder` | `snippets::OpenFolder` | `snippets: open folder` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-svg-OpenFollowingPreview` | `svg::OpenFollowingPreview` | `svg: open following preview` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-syntax_tree_view-UseActiveEditor` | `syntax_tree_view::UseActiveEditor` | `syntax tree view: use active editor` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-tab_switcher-OpenInActivePane` | `tab_switcher::OpenInActivePane` | `tab switcher: open in active pane` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-tab_switcher-ToggleAll` | `tab_switcher::ToggleAll` | `tab switcher: toggle all` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-terminal-RenameTerminal` | `terminal::RenameTerminal` | `terminal: rename terminal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-terminal-ScrollHalfPageDown` | `terminal::ScrollHalfPageDown` | `terminal: scroll half page down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-terminal-ScrollHalfPageUp` | `terminal::ScrollHalfPageUp` | `terminal: scroll half page up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-terminal-SearchTest` | `terminal::SearchTest` | `terminal: search test` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-terminal-SelectAll` | `terminal::SelectAll` | `terminal: select all` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-terminal_panel-ToggleFocus` | `terminal_panel::ToggleFocus` | `terminal panel: toggle focus` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-text_finder-Fold` | `text_finder::Fold` | `text finder: fold` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-text_finder-ToProjectSearch` | `text_finder::ToProjectSearch` | `text finder: to project search` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-text_finder-ToggleFoldAll` | `text_finder::ToggleFoldAll` | `text finder: toggle fold all` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-text_finder-Unfold` | `text_finder::Unfold` | `text finder: unfold` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-theme_selector-Reload` | `theme_selector::Reload` | `theme selector: reload` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-toolchain-Select` | `toolchain::Select` | `toolchain: select` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-variable_list-GoToMemory` | `variable_list::GoToMemory` | `variable list: go to memory` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-AngleBrackets` | `vim::AngleBrackets` | `vim: angle brackets` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-AnyBrackets` | `vim::AnyBrackets` | `vim: any brackets` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-AnyPair` | `vim::AnyPair` | `vim: any pair` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-AnyQuotes` | `vim::AnyQuotes` | `vim: any quotes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Argument` | `vim::Argument` | `vim: argument` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ArgumentRequired` | `vim::ArgumentRequired` | `vim: argument required` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-AutoIndent` | `vim::AutoIndent` | `vim: auto indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-BackQuotes` | `vim::BackQuotes` | `vim: back quotes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ChangeCase` | `vim::ChangeCase` | `vim: change case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ChangeListNewer` | `vim::ChangeListNewer` | `vim: change list newer` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ChangeListOlder` | `vim::ChangeListOlder` | `vim: change list older` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ChangeToEndOfLine` | `vim::ChangeToEndOfLine` | `vim: change to end of line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Class` | `vim::Class` | `vim: class` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ClearExchange` | `vim::ClearExchange` | `vim: clear exchange` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ClearOperators` | `vim::ClearOperators` | `vim: clear operators` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ColumnLeft` | `vim::ColumnLeft` | `vim: column left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ColumnRight` | `vim::ColumnRight` | `vim: column right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Comment` | `vim::Comment` | `vim: comment` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ConvertToLowerCase` | `vim::ConvertToLowerCase` | `vim: convert to lower case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ConvertToRot13` | `vim::ConvertToRot13` | `vim: convert to rot13` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ConvertToRot47` | `vim::ConvertToRot47` | `vim: convert to rot47` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ConvertToUpperCase` | `vim::ConvertToUpperCase` | `vim: convert to upper case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-CountCommand` | `vim::CountCommand` | `vim: count command` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-CurlyBrackets` | `vim::CurlyBrackets` | `vim: curly brackets` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-CurrentLine` | `vim::CurrentLine` | `vim: current line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Decrement` | `vim::Decrement` | `vim: decrement` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-DeleteLeft` | `vim::DeleteLeft` | `vim: delete left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-DeleteRight` | `vim::DeleteRight` | `vim: delete right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-DeleteToEndOfLine` | `vim::DeleteToEndOfLine` | `vim: delete to end of line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-DoubleQuotes` | `vim::DoubleQuotes` | `vim: double quotes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Down` | `vim::Down` | `vim: down` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-EndOfDocument` | `vim::EndOfDocument` | `vim: end of document` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-EndOfLine` | `vim::EndOfLine` | `vim: end of line` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-EndOfLineDownward` | `vim::EndOfLineDownward` | `vim: end of line downward` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-EndOfParagraph` | `vim::EndOfParagraph` | `vim: end of paragraph` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-EndRepeat` | `vim::EndRepeat` | `vim: end repeat` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Enter` | `vim::Enter` | `vim: enter` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-EntireFile` | `vim::EntireFile` | `vim: entire file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Exchange` | `vim::Exchange` | `vim: exchange` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-FindCommand` | `vim::FindCommand` | `vim: find command` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-FirstNonWhitespace` | `vim::FirstNonWhitespace` | `vim: first non whitespace` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-GoToColumn` | `vim::GoToColumn` | `vim: go to column` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-GoToNextReference` | `vim::GoToNextReference` | `vim: go to next reference` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-GoToPercentage` | `vim::GoToPercentage` | `vim: go to percentage` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-GoToPreviousReference` | `vim::GoToPreviousReference` | `vim: go to previous reference` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-GoToPreviousTab` | `vim::GoToPreviousTab` | `vim: go to previous tab` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-GoToTab` | `vim::GoToTab` | `vim: go to tab` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HalfPageLeft` | `vim::HalfPageLeft` | `vim: half page left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HalfPageRight` | `vim::HalfPageRight` | `vim: half page right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixAppend` | `vim::HelixAppend` | `vim: helix append` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixCollapseSelection` | `vim::HelixCollapseSelection` | `vim: helix collapse selection` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixDelete` | `vim::HelixDelete` | `vim: helix delete` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixDuplicateAbove` | `vim::HelixDuplicateAbove` | `vim: helix duplicate above` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixDuplicateBelow` | `vim::HelixDuplicateBelow` | `vim: helix duplicate below` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixGotoLastModification` | `vim::HelixGotoLastModification` | `vim: helix goto last modification` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixGotoLine` | `vim::HelixGotoLine` | `vim: helix goto line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixInsert` | `vim::HelixInsert` | `vim: helix insert` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixInsertEndOfLine` | `vim::HelixInsertEndOfLine` | `vim: helix insert end of line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixJumpToWord` | `vim::HelixJumpToWord` | `vim: helix jump to word` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixKeepNewestSelection` | `vim::HelixKeepNewestSelection` | `vim: helix keep newest selection` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixPaste` | `vim::HelixPaste` | `vim: helix paste` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixSelectLine` | `vim::HelixSelectLine` | `vim: helix select line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixSelectNext` | `vim::HelixSelectNext` | `vim: helix select next` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixSelectPrevious` | `vim::HelixSelectPrevious` | `vim: helix select previous` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixSelectRegex` | `vim::HelixSelectRegex` | `vim: helix select regex` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixSubstitute` | `vim::HelixSubstitute` | `vim: helix substitute` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixSubstituteNoYank` | `vim::HelixSubstituteNoYank` | `vim: helix substitute no yank` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixTrimSelections` | `vim::HelixTrimSelections` | `vim: helix trim selections` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-HelixYank` | `vim::HelixYank` | `vim: helix yank` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Increment` | `vim::Increment` | `vim: increment` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Indent` | `vim::Indent` | `vim: indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-IndentObj` | `vim::IndentObj` | `vim: indent obj` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InnerObject` | `vim::InnerObject` | `vim: inner object` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertAfter` | `vim::InsertAfter` | `vim: insert after` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertAtPrevious` | `vim::InsertAtPrevious` | `vim: insert at previous` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertBefore` | `vim::InsertBefore` | `vim: insert before` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertEmptyLineAbove` | `vim::InsertEmptyLineAbove` | `vim: insert empty line above` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertEmptyLineBelow` | `vim::InsertEmptyLineBelow` | `vim: insert empty line below` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertEndOfLine` | `vim::InsertEndOfLine` | `vim: insert end of line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertFirstNonWhitespace` | `vim::InsertFirstNonWhitespace` | `vim: insert first non whitespace` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertFromAbove` | `vim::InsertFromAbove` | `vim: insert from above` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertFromBelow` | `vim::InsertFromBelow` | `vim: insert from below` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertLineAbove` | `vim::InsertLineAbove` | `vim: insert line above` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-InsertLineBelow` | `vim::InsertLineBelow` | `vim: insert line below` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-JoinLines` | `vim::JoinLines` | `vim: join lines` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-JoinLinesNoWhitespace` | `vim::JoinLinesNoWhitespace` | `vim: join lines no whitespace` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Left` | `vim::Left` | `vim: left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-LineDown` | `vim::LineDown` | `vim: line down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-LineUp` | `vim::LineUp` | `vim: line up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Literal` | `vim::Literal` | `vim: literal` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Matching` | `vim::Matching` | `vim: matching` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MaximizePane` | `vim::MaximizePane` | `vim: maximize pane` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MenuSelectNext` | `vim::MenuSelectNext` | `vim: menu select next` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MenuSelectPrevious` | `vim::MenuSelectPrevious` | `vim: menu select previous` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Method` | `vim::Method` | `vim: method` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MiddleOfLine` | `vim::MiddleOfLine` | `vim: middle of line` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MiniBrackets` | `vim::MiniBrackets` | `vim: mini brackets` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MiniQuotes` | `vim::MiniQuotes` | `vim: mini quotes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MoveToNext` | `vim::MoveToNext` | `vim: move to next` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MoveToNextMatch` | `vim::MoveToNextMatch` | `vim: move to next match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MoveToPrevious` | `vim::MoveToPrevious` | `vim: move to previous` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-MoveToPreviousMatch` | `vim::MoveToPreviousMatch` | `vim: move to previous match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextComment` | `vim::NextComment` | `vim: next comment` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextGreaterIndent` | `vim::NextGreaterIndent` | `vim: next greater indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextLesserIndent` | `vim::NextLesserIndent` | `vim: next lesser indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextLineStart` | `vim::NextLineStart` | `vim: next line start` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextMethodEnd` | `vim::NextMethodEnd` | `vim: next method end` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextMethodStart` | `vim::NextMethodStart` | `vim: next method start` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextSameIndent` | `vim::NextSameIndent` | `vim: next same indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextSectionEnd` | `vim::NextSectionEnd` | `vim: next section end` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextSectionStart` | `vim::NextSectionStart` | `vim: next section start` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextSubwordEnd` | `vim::NextSubwordEnd` | `vim: next subword end` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextSubwordStart` | `vim::NextSubwordStart` | `vim: next subword start` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextWordEnd` | `vim::NextWordEnd` | `vim: next word end` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NextWordStart` | `vim::NextWordStart` | `vim: next word start` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-NormalBefore` | `vim::NormalBefore` | `vim: normal before` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Number` | `vim::Number` | `vim: number` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-OpenDefaultKeymap` | `vim::OpenDefaultKeymap` | `vim: open default keymap` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-OtherEnd` | `vim::OtherEnd` | `vim: other end` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-OtherEndRowAware` | `vim::OtherEndRowAware` | `vim: other end row aware` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Outdent` | `vim::Outdent` | `vim: outdent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PageDown` | `vim::PageDown` | `vim: page down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PageUp` | `vim::PageUp` | `vim: page up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Paragraph` | `vim::Paragraph` | `vim: paragraph` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Parentheses` | `vim::Parentheses` | `vim: parentheses` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Paste` | `vim::Paste` | `vim: paste` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousComment` | `vim::PreviousComment` | `vim: previous comment` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousGreaterIndent` | `vim::PreviousGreaterIndent` | `vim: previous greater indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousLesserIndent` | `vim::PreviousLesserIndent` | `vim: previous lesser indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousLineStart` | `vim::PreviousLineStart` | `vim: previous line start` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousMethodEnd` | `vim::PreviousMethodEnd` | `vim: previous method end` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousMethodStart` | `vim::PreviousMethodStart` | `vim: previous method start` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousSameIndent` | `vim::PreviousSameIndent` | `vim: previous same indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousSectionEnd` | `vim::PreviousSectionEnd` | `vim: previous section end` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousSectionStart` | `vim::PreviousSectionStart` | `vim: previous section start` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousSubwordEnd` | `vim::PreviousSubwordEnd` | `vim: previous subword end` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousSubwordStart` | `vim::PreviousSubwordStart` | `vim: previous subword start` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousWordEnd` | `vim::PreviousWordEnd` | `vim: previous word end` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PreviousWordStart` | `vim::PreviousWordStart` | `vim: previous word start` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushAddSurrounds` | `vim::PushAddSurrounds` | `vim: push add surrounds` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushAutoIndent` | `vim::PushAutoIndent` | `vim: push auto indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushChange` | `vim::PushChange` | `vim: push change` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushChangeSurrounds` | `vim::PushChangeSurrounds` | `vim: push change surrounds` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushDelete` | `vim::PushDelete` | `vim: push delete` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushDeleteSurrounds` | `vim::PushDeleteSurrounds` | `vim: push delete surrounds` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushDigraph` | `vim::PushDigraph` | `vim: push digraph` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushFindBackward` | `vim::PushFindBackward` | `vim: push find backward` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushFindForward` | `vim::PushFindForward` | `vim: push find forward` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushForcedMotion` | `vim::PushForcedMotion` | `vim: push forced motion` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushHelixMatch` | `vim::PushHelixMatch` | `vim: push helix match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushHelixNext` | `vim::PushHelixNext` | `vim: push helix next` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushHelixPrevious` | `vim::PushHelixPrevious` | `vim: push helix previous` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushHelixSurroundAdd` | `vim::PushHelixSurroundAdd` | `vim: push helix surround add` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushHelixSurroundDelete` | `vim::PushHelixSurroundDelete` | `vim: push helix surround delete` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushHelixSurroundReplace` | `vim::PushHelixSurroundReplace` | `vim: push helix surround replace` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushIndent` | `vim::PushIndent` | `vim: push indent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushJump` | `vim::PushJump` | `vim: push jump` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushLiteral` | `vim::PushLiteral` | `vim: push literal` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushLowercase` | `vim::PushLowercase` | `vim: push lowercase` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushMark` | `vim::PushMark` | `vim: push mark` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushObject` | `vim::PushObject` | `vim: push object` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushOppositeCase` | `vim::PushOppositeCase` | `vim: push opposite case` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushOutdent` | `vim::PushOutdent` | `vim: push outdent` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushRecordRegister` | `vim::PushRecordRegister` | `vim: push record register` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushRegister` | `vim::PushRegister` | `vim: push register` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushReplace` | `vim::PushReplace` | `vim: push replace` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushReplaceWithRegister` | `vim::PushReplaceWithRegister` | `vim: push replace with register` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushReplayRegister` | `vim::PushReplayRegister` | `vim: push replay register` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushRewrap` | `vim::PushRewrap` | `vim: push rewrap` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushRot13` | `vim::PushRot13` | `vim: push rot13` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushRot47` | `vim::PushRot47` | `vim: push rot47` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushShellCommand` | `vim::PushShellCommand` | `vim: push shell command` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushSneak` | `vim::PushSneak` | `vim: push sneak` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushSneakBackward` | `vim::PushSneakBackward` | `vim: push sneak backward` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushToggleBlockComments` | `vim::PushToggleBlockComments` | `vim: push toggle block comments` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushToggleComments` | `vim::PushToggleComments` | `vim: push toggle comments` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushUppercase` | `vim::PushUppercase` | `vim: push uppercase` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-PushYank` | `vim::PushYank` | `vim: push yank` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Quotes` | `vim::Quotes` | `vim: quotes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Redo` | `vim::Redo` | `vim: redo` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Repeat` | `vim::Repeat` | `vim: repeat` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-RepeatFind` | `vim::RepeatFind` | `vim: repeat find` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-RepeatFindReversed` | `vim::RepeatFindReversed` | `vim: repeat find reversed` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ReplayLastRecording` | `vim::ReplayLastRecording` | `vim: replay last recording` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ResetPaneSizes` | `vim::ResetPaneSizes` | `vim: reset pane sizes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ResizePaneDown` | `vim::ResizePaneDown` | `vim: resize pane down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ResizePaneLeft` | `vim::ResizePaneLeft` | `vim: resize pane left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ResizePaneRight` | `vim::ResizePaneRight` | `vim: resize pane right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ResizePaneUp` | `vim::ResizePaneUp` | `vim: resize pane up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-RestoreVisualSelection` | `vim::RestoreVisualSelection` | `vim: restore visual selection` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Rewrap` | `vim::Rewrap` | `vim: rewrap` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Right` | `vim::Right` | `vim: right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ScrollDown` | `vim::ScrollDown` | `vim: scroll down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ScrollUp` | `vim::ScrollUp` | `vim: scroll up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Search` | `vim::Search` | `vim: search` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SearchSubmit` | `vim::SearchSubmit` | `vim: search submit` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SearchUnderCursor` | `vim::SearchUnderCursor` | `vim: search under cursor` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SearchUnderCursorPrevious` | `vim::SearchUnderCursorPrevious` | `vim: search under cursor previous` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectLargerSyntaxNode` | `vim::SelectLargerSyntaxNode` | `vim: select larger syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectNext` | `vim::SelectNext` | `vim: select next` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectNextMatch` | `vim::SelectNextMatch` | `vim: select next match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectNextSyntaxNode` | `vim::SelectNextSyntaxNode` | `vim: select next syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectPrevious` | `vim::SelectPrevious` | `vim: select previous` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectPreviousMatch` | `vim::SelectPreviousMatch` | `vim: select previous match` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectPreviousSyntaxNode` | `vim::SelectPreviousSyntaxNode` | `vim: select previous syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectRegister` | `vim::SelectRegister` | `vim: select register` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SelectSmallerSyntaxNode` | `vim::SelectSmallerSyntaxNode` | `vim: select smaller syntax node` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Sentence` | `vim::Sentence` | `vim: sentence` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SentenceBackward` | `vim::SentenceBackward` | `vim: sentence backward` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SentenceForward` | `vim::SentenceForward` | `vim: sentence forward` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ShellCommand` | `vim::ShellCommand` | `vim: shell command` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ShowLocation` | `vim::ShowLocation` | `vim: show location` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SquareBrackets` | `vim::SquareBrackets` | `vim: square brackets` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-StartOfDocument` | `vim::StartOfDocument` | `vim: start of document` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-StartOfLine` | `vim::StartOfLine` | `vim: start of line` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-StartOfLineDownward` | `vim::StartOfLineDownward` | `vim: start of line downward` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-StartOfParagraph` | `vim::StartOfParagraph` | `vim: start of paragraph` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Substitute` | `vim::Substitute` | `vim: substitute` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SubstituteLine` | `vim::SubstituteLine` | `vim: substitute line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Subword` | `vim::Subword` | `vim: subword` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToHelixNormalMode` | `vim::SwitchToHelixNormalMode` | `vim: switch to helix normal mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToInsertMode` | `vim::SwitchToInsertMode` | `vim: switch to insert mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToNormalMode` | `vim::SwitchToNormalMode` | `vim: switch to normal mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToReplaceMode` | `vim::SwitchToReplaceMode` | `vim: switch to replace mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToVisualBlockMode` | `vim::SwitchToVisualBlockMode` | `vim: switch to visual block mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToVisualLineMode` | `vim::SwitchToVisualLineMode` | `vim: switch to visual line mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-SwitchToVisualMode` | `vim::SwitchToVisualMode` | `vim: switch to visual mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Tab` | `vim::Tab` | `vim: tab` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Tag` | `vim::Tag` | `vim: tag` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-TemporaryNormal` | `vim::TemporaryNormal` | `vim: temporary normal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleBlockComments` | `vim::ToggleBlockComments` | `vim: toggle block comments` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleComments` | `vim::ToggleComments` | `vim: toggle comments` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleMarksView` | `vim::ToggleMarksView` | `vim: toggle marks view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleProjectPanelFocus` | `vim::ToggleProjectPanelFocus` | `vim: toggle project panel focus` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleRecord` | `vim::ToggleRecord` | `vim: toggle record` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleRegistersView` | `vim::ToggleRegistersView` | `vim: toggle registers view` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleReplace` | `vim::ToggleReplace` | `vim: toggle replace` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleVisual` | `vim::ToggleVisual` | `vim: toggle visual` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleVisualBlock` | `vim::ToggleVisualBlock` | `vim: toggle visual block` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-ToggleVisualLine` | `vim::ToggleVisualLine` | `vim: toggle visual line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Undo` | `vim::Undo` | `vim: undo` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-UndoLastLine` | `vim::UndoLastLine` | `vim: undo last line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-UndoReplace` | `vim::UndoReplace` | `vim: undo replace` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-UnmatchedBackward` | `vim::UnmatchedBackward` | `vim: unmatched backward` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-UnmatchedForward` | `vim::UnmatchedForward` | `vim: unmatched forward` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Up` | `vim::Up` | `vim: up` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VerticalBars` | `vim::VerticalBars` | `vim: vertical bars` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualCommand` | `vim::VisualCommand` | `vim: visual command` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualDelete` | `vim::VisualDelete` | `vim: visual delete` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualDeleteLine` | `vim::VisualDeleteLine` | `vim: visual delete line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualInsertEndOfLine` | `vim::VisualInsertEndOfLine` | `vim: visual insert end of line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualInsertFirstNonWhiteSpace` | `vim::VisualInsertFirstNonWhiteSpace` | `vim: visual insert first non white space` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualYank` | `vim::VisualYank` | `vim: visual yank` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-VisualYankLine` | `vim::VisualYankLine` | `vim: visual yank line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-WindowBottom` | `vim::WindowBottom` | `vim: window bottom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-WindowMiddle` | `vim::WindowMiddle` | `vim: window middle` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-WindowTop` | `vim::WindowTop` | `vim: window top` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Word` | `vim::Word` | `vim: word` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-WrappingLeft` | `vim::WrappingLeft` | `vim: wrapping left` | Present | None | Present | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-WrappingRight` | `vim::WrappingRight` | `vim: wrapping right` | Present | None | Present | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-Yank` | `vim::Yank` | `vim: yank` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-YankLine` | `vim::YankLine` | `vim: yank line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-vim-YankToEndOfLine` | `vim::YankToEndOfLine` | `vim: yank to end of line` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-window-MergeAllWindows` | `window::MergeAllWindows` | `window: merge all windows` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-window-MoveTabToNewWindow` | `window::MoveTabToNewWindow` | `window: move tab to new window` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-window-ShowNextWindowTab` | `window::ShowNextWindowTab` | `window: show next window tab` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-window-ShowPreviousWindowTab` | `window::ShowPreviousWindowTab` | `window: show previous window tab` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ActivateLastPane` | `workspace::ActivateLastPane` | `workspace: activate last pane` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ActivateNextPane` | `workspace::ActivateNextPane` | `workspace: activate next pane` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ActivateNextWindow` | `workspace::ActivateNextWindow` | `workspace: activate next window` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ActivatePreviousPane` | `workspace::ActivatePreviousPane` | `workspace: activate previous pane` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ActivatePreviousWindow` | `workspace::ActivatePreviousWindow` | `workspace: activate previous window` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ClearAllNotifications` | `workspace::ClearAllNotifications` | `workspace: clear all notifications` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ClearBookmarks` | `workspace::ClearBookmarks` | `workspace: clear bookmarks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ClearNavigationHistory` | `workspace::ClearNavigationHistory` | `workspace: clear navigation history` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ClearTrustedWorktrees` | `workspace::ClearTrustedWorktrees` | `workspace: clear trusted worktrees` | Present | None | None | Clear persisted trusted-worktree paths; new projects return to Restricted Mode and already-open projects require restart. Invocation route remains unverified. | Workspace; no Labonair trust store or route | Pending (runtime discovery and reset behavior not observed): [Workspace worktree-security classification](action-classification-workspace-worktree-security.md) |
| `ACT-REGISTRY-LINUX-workspace-CloseAllDocks` | `workspace::CloseAllDocks` | `workspace: close all docks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-CloseItemInAllPanes` | `workspace::CloseItemInAllPanes` | `workspace: close item in all panes` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-CloseProject` | `workspace::CloseProject` | `workspace: close project` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-Feedback` | `workspace::Feedback` | `workspace: feedback` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-FocusCenterPane` | `workspace::FocusCenterPane` | `workspace: focus center pane` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-FormatAndSave` | `workspace::FormatAndSave` | `workspace: format and save` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MoveFocusedPanelToNextPosition` | `workspace::MoveFocusedPanelToNextPosition` | `workspace: move focused panel to next position` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MoveItemToPane` | `workspace::MoveItemToPane` | `workspace: move item to pane` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MoveItemToPaneInDirection` | `workspace::MoveItemToPaneInDirection` | `workspace: move item to pane in direction` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MovePaneDown` | `workspace::MovePaneDown` | `workspace: move pane down` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MovePaneLeft` | `workspace::MovePaneLeft` | `workspace: move pane left` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MovePaneRight` | `workspace::MovePaneRight` | `workspace: move pane right` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-MovePaneUp` | `workspace::MovePaneUp` | `workspace: move pane up` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-NewCenterTerminal` | `workspace::NewCenterTerminal` | `workspace: new center terminal` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-NewFileSplit` | `workspace::NewFileSplit` | `workspace: new file split` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-NewFileSplitHorizontal` | `workspace::NewFileSplitHorizontal` | `workspace: new file split horizontal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-NewFileSplitVertical` | `workspace::NewFileSplitVertical` | `workspace: new file split vertical` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-NewSearch` | `workspace::NewSearch` | `workspace: new search` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-OpenComponentPreview` | `workspace::OpenComponentPreview` | `workspace: open component preview` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-OpenInTerminal` | `workspace::OpenInTerminal` | `workspace: open in terminal` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-OpenTerminal` | `workspace::OpenTerminal` | `workspace: open terminal` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-Reload` | `workspace::Reload` | `workspace: reload` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ReloadActiveItem` | `workspace::ReloadActiveItem` | `workspace: reload active item` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ResetPaneSizes` | `workspace::ResetPaneSizes` | `workspace: reset pane sizes` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-RestoreBanner` | `workspace::RestoreBanner` | `workspace: restore banner` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-SendKeystrokes` | `workspace::SendKeystrokes` | `workspace: send keystrokes` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ShutdownDebugAdapters` | `workspace::ShutdownDebugAdapters` | `workspace: shutdown debug adapters` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-SuppressNotification` | `workspace::SuppressNotification` | `workspace: suppress notification` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-SwapPaneAdjacent` | `workspace::SwapPaneAdjacent` | `workspace: swap pane adjacent` | None | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleCenteredLayout` | `workspace::ToggleCenteredLayout` | `workspace: toggle centered layout` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleEditPrediction` | `workspace::ToggleEditPrediction` | `workspace: toggle edit prediction` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleEditorZoom` | `workspace::ToggleEditorZoom` | `workspace: toggle editor zoom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleExpandItem` | `workspace::ToggleExpandItem` | `workspace: toggle expand item` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleHelixMode` | `workspace::ToggleHelixMode` | `workspace: toggle helix mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleReadOnlyFile` | `workspace::ToggleReadOnlyFile` | `workspace: toggle read only file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ToggleVimMode` | `workspace::ToggleVimMode` | `workspace: toggle vim mode` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-UseAgenticLayout` | `workspace::UseAgenticLayout` | `workspace: use agentic layout` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-UseClassicLayout` | `workspace::UseClassicLayout` | `workspace: use classic layout` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ZoomIn` | `workspace::ZoomIn` | `workspace: zoom in` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-workspace-ZoomOut` | `workspace::ZoomOut` | `workspace: zoom out` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-About` | `zed::About` | `zed: about` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-AcpRegistry` | `zed::AcpRegistry` | `zed: acp registry` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-CopyInstalledExtensionsIntoClipboard` | `zed::CopyInstalledExtensionsIntoClipboard` | `zed: copy installed extensions into clipboard` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-CopySystemSpecsIntoClipboard` | `zed::CopySystemSpecsIntoClipboard` | `zed: copy system specs into clipboard` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-DebugElements` | `zed::DebugElements` | `zed: debug elements` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-GetMerch` | `zed::GetMerch` | `zed: get merch` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ImportCursorSettings` | `zed::ImportCursorSettings` | `zed: import cursor settings` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ImportVsCodeSettings` | `zed::ImportVsCodeSettings` | `zed: import vs code settings` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-InstallDevExtension` | `zed::InstallDevExtension` | `zed: install dev extension` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-NoAction` | `zed::NoAction` | `zed: no action` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenAccountSettings` | `zed::OpenAccountSettings` | `zed: open account settings` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenBrowser` | `zed::OpenBrowser` | `zed: open browser` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenDebugTasks` | `zed::OpenDebugTasks` | `zed: open debug tasks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenDefaultKeymap` | `zed::OpenDefaultKeymap` | `zed: open default keymap` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenDefaultSettings` | `zed::OpenDefaultSettings` | `zed: open default settings` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenDocs` | `zed::OpenDocs` | `zed: open docs` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenLicenses` | `zed::OpenLicenses` | `zed: open licenses` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenLog` | `zed::OpenLog` | `zed: open log` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenOnboarding` | `zed::OpenOnboarding` | `zed: open onboarding` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenPerformanceProfiler` | `zed::OpenPerformanceProfiler` | `zed: open performance profiler` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenProjectSettings` | `zed::OpenProjectSettings` | `zed: open project settings` | Present | None | Present | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenProjectSettingsFile` | `zed::OpenProjectSettingsFile` | `zed: open project settings file` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenProjectTasks` | `zed::OpenProjectTasks` | `zed: open project tasks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenServerSettings` | `zed::OpenServerSettings` | `zed: open server settings` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenSettingsAt` | `zed::OpenSettingsAt` | `zed: open settings at` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenSettingsPage` | `zed::OpenSettingsPage` | `zed: open settings page` | None | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenStatusPage` | `zed::OpenStatusPage` | `zed: open status page` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenTasks` | `zed::OpenTasks` | `zed: open tasks` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenTelemetryLog` | `zed::OpenTelemetryLog` | `zed: open telemetry log` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenZedRepo` | `zed::OpenZedRepo` | `zed: open zed repo` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-OpenZedUrl` | `zed::OpenZedUrl` | `zed: open zed url` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-RebuildDevExtension` | `zed::RebuildDevExtension` | `zed: rebuild dev extension` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ReloadExtensions` | `zed::ReloadExtensions` | `zed: reload extensions` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ResetAllZoom` | `zed::ResetAllZoom` | `zed: reset all zoom` | Present | Present | None | Reset UI, buffer, Agent UI, and Agent buffer sizes through View > Reset All Zoom with `persist:false`; Git Commit and Markdown Preview scopes are separate: [font-size increase and reset classification](action-classification-font-size-increase-reset.md) | Themes (`labonair-theme`) owns the four typography scopes; the existing View menu is the user entry point | The registered action and menu route have no Labonair handler; R08-002 removed the previous no-op Zoom routes | Pending (runtime interaction not observed) |
| `ACT-REGISTRY-LINUX-zed-ResetDatabase` | `zed::ResetDatabase` | `zed: reset database` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-RevealLogInFileManager` | `zed::RevealLogInFileManager` | `zed: reveal log in file manager` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ShowAll` | `zed::ShowAll` | `zed: show all` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ShowDefaultSemanticTokenRules` | `zed::ShowDefaultSemanticTokenRules` | `zed: show default semantic token rules` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ShowUpdateNotification` | `zed::ShowUpdateNotification` | `zed: show update notification` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ShowWelcome` | `zed::ShowWelcome` | `zed: show welcome` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-TestCrash` | `zed::TestCrash` | `zed: test crash` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-TestPanic` | `zed::TestPanic` | `zed: test panic` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-ToggleBaseKeymapSelector` | `zed::ToggleBaseKeymapSelector` | `zed: toggle base keymap selector` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-Unbind` | `zed::Unbind` | `zed: unbind` | Present | Present | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed-Zoom` | `zed::Zoom` | `zed: zoom` | Present | None | None | Pending | Pending | Pending |
| `ACT-REGISTRY-LINUX-zed_predict_onboarding-OpenZedPredictOnboarding` | `zed_predict_onboarding::OpenZedPredictOnboarding` | `zed predict onboarding: open zed predict onboarding` | Present | None | None | Pending | Pending | Pending |
