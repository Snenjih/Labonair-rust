import { describe, expect, it } from "vitest";
import { isPlainTextInputFocused } from "./useGlobalShortcuts";

describe("isPlainTextInputFocused", () => {
  it("is true for a real <input>, e.g. TabRenameInput", () => {
    const input = document.createElement("input");
    expect(isPlainTextInputFocused(input)).toBe(true);
  });

  it("is true for a <textarea>", () => {
    const textarea = document.createElement("textarea");
    expect(isPlainTextInputFocused(textarea)).toBe(true);
  });

  it("is false for null (nothing focused)", () => {
    expect(isPlainTextInputFocused(null)).toBe(false);
  });

  it("is false for a plain div", () => {
    const div = document.createElement("div");
    expect(isPlainTextInputFocused(div)).toBe(false);
  });

  // CodeMirror's editor surface is contenteditable, not <textarea> — a
  // blanket contenteditable exclusion would regress ⌘F (search.focus)
  // while the cursor is in the code editor, so this must stay false here.
  it("is false for a contenteditable div (e.g. the CodeMirror editor surface)", () => {
    const div = document.createElement("div");
    div.contentEditable = "true";
    expect(isPlainTextInputFocused(div)).toBe(false);
  });

  it("is false for a button", () => {
    const button = document.createElement("button");
    expect(isPlainTextInputFocused(button)).toBe(false);
  });

  // xterm.js keeps a hidden <textarea class="xterm-helper-textarea"> focused
  // offscreen whenever a terminal pane has focus (its mechanism for
  // capturing raw keyboard/IME input) — without this exclusion, every global
  // shortcut (including tab switching) would silently stop matching the
  // moment a terminal is focused, since it's a real HTMLTextAreaElement.
  it("is false for xterm's hidden helper textarea", () => {
    const textarea = document.createElement("textarea");
    textarea.classList.add("xterm-helper-textarea");
    expect(isPlainTextInputFocused(textarea)).toBe(false);
  });
});
