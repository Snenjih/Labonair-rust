import { describe, expect, it, vi } from "vitest";

// scrollback.ts also imports terminalSessionRegistry.ts for saveAllScrollbacks/
// flushDormantScrollback (unrelated to closeDanglingAltScreen, the pure
// function under test here) — that module transitively pulls in rendererPool.ts's
// xterm addon imports, which fail to resolve in this environment
// (@xterm/addon-ligatures has a broken `main` field, a pre-existing issue
// unrelated to this fix). Stub it out so this file can test the pure
// function in isolation without needing that chain to resolve.
vi.mock("@/modules/terminal/lib/terminalSessionRegistry", () => ({
  commitDormantFlush: vi.fn(),
  getAllSessionIds: vi.fn(() => []),
  peekDormantAnsi: vi.fn(() => null),
}));

const { closeDanglingAltScreen } = await import("./scrollback");

const ENTER = "\x1b[?1049h";
const EXIT = "\x1b[?1049l";

describe("closeDanglingAltScreen", () => {
  it("returns plain scrollback with no alt-screen markers unchanged", () => {
    const ansi = "$ ls\r\nfoo.txt\r\n$ cat foo.txt\r\nhello\r\n$ ";
    expect(closeDanglingAltScreen(ansi)).toBe(ansi);
  });

  it("appends an exit sequence when the blob ends with a dangling alt-screen entry", () => {
    const ansi = `$ htop\r\n${ENTER}\x1b[H<htop frame content>`;
    expect(closeDanglingAltScreen(ansi)).toBe(ansi + EXIT);
  });

  it("leaves a balanced enter/exit pair unchanged", () => {
    const ansi = `$ htop\r\n${ENTER}\x1b[H<htop frame content>${EXIT}$ echo done\r\ndone\r\n`;
    expect(closeDanglingAltScreen(ansi)).toBe(ansi);
  });

  it("appends an exit sequence for re-entrant TUI use that ends dangling", () => {
    const ansi = `${ENTER}first frame${EXIT}$ vim\r\n${ENTER}second frame`;
    expect(closeDanglingAltScreen(ansi)).toBe(ansi + EXIT);
  });

  it("returns an empty string unchanged", () => {
    expect(closeDanglingAltScreen("")).toBe("");
  });
});
