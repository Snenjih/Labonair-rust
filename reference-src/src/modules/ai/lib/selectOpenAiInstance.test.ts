import { describe, expect, it } from "vitest";
import type { ProviderInstance } from "../config";
import { selectOpenAiInstanceKey } from "./selectOpenAiInstance";

function makeInstance(id: string, providerId: ProviderInstance["providerId"]): ProviderInstance {
  return { id, providerId, name: id };
}

describe("selectOpenAiInstanceKey", () => {
  it("returns null when no instances are configured", () => {
    expect(selectOpenAiInstanceKey([], {})).toBeNull();
  });

  it("returns null when an openai instance exists but has no key", () => {
    const instances = [makeInstance("inst-1", "openai")];
    expect(selectOpenAiInstanceKey(instances, { "inst-1": null })).toBeNull();
  });

  it("returns the key of the first keyed openai instance", () => {
    const instances = [makeInstance("inst-1", "openai"), makeInstance("inst-2", "openai")];
    expect(selectOpenAiInstanceKey(instances, { "inst-1": "sk-abc", "inst-2": "sk-def" })).toBe("sk-abc");
  });

  it("skips an unkeyed openai instance and picks the next keyed one", () => {
    const instances = [makeInstance("inst-1", "openai"), makeInstance("inst-2", "openai")];
    expect(selectOpenAiInstanceKey(instances, { "inst-1": null, "inst-2": "sk-def" })).toBe("sk-def");
  });

  it("ignores non-openai instances even with a key (e.g. openai-compatible/local)", () => {
    const instances = [makeInstance("inst-1", "openai-compatible"), makeInstance("inst-2", "ollama")];
    expect(
      selectOpenAiInstanceKey(instances, { "inst-1": "sk-abc", "inst-2": "unused-but-keyless" }),
    ).toBeNull();
  });
});
