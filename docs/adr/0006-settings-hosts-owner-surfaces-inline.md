# ADR 0006 — Settings may host owner surfaces inline (Themes page)

* **Status:** Accepted
* **Date:** 2026-11-05
* **Deciders:** project owner and implementation team
* **Related:** [`../settings-guidelines.md`](../settings-guidelines.md), [`capabilities.md`](../capabilities.md), [`../design-system.md`](../design-system.md)

## Context

AGENTS.md rule 9 states that Settings contain values only and that themes,
icon themes, hosts, and similar concerns belong to their own modules. The
product now wants one Settings window with a Themes category (colour themes,
icon themes, background options) instead of pop-ups and palette-only pickers.

## Decision

1. Settings may render owner surfaces inline. A `SettingsSurface` registered by
   its owner gains an inline render mode; the Settings window embeds the
   owner's view in a page section. Ownership of state, persistence, and
   behaviour stays with the owner (`theme`, `background`, ...). Settings
   stores no theme or background data of its own.
2. Settings pages group values and inline owner surfaces into cards. The
   category tree is: General, Appearance, Themes, Editor, Languages & Tools,
   Search & Files, Terminal, then the owner pages Keymap and Hosts.
3. The background options modal is removed; the same owner view is shown inline
   on the Themes page.
4. Every theme family must provide a light and a dark variant. Incomplete
   family files are rejected at load with a notification.
5. Rule 9 is read as: Settings do not own non-settings state, and no feature
   may add a parallel settings category; hosting an owner surface inline is
   allowed.
