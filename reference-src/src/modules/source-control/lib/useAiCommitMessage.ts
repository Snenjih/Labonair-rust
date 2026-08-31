import { generateText } from "ai";
import { useState } from "react";
import { buildModel } from "@/modules/ai/lib/agent";
import { EMPTY_PROVIDER_KEYS } from "@/modules/ai/lib/keyring";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useProvidersStore } from "@/modules/ai/store/providersStore";
import { git } from "./gitInvoke";

const COMMIT_MSG_SYSTEM_PROMPT = `You are an expert software engineer writing a git commit message for a set of staged changes, given a unified diff. Optionally you are also given a list of changed files and recent commit subjects from this repository for style reference. Produce exactly one commit message that follows the Conventional Commits specification.

STRUCTURE
- Subject line: \`type(scope): description\`
  - type is one of: feat, fix, docs, style, refactor, perf, test, build, ci, chore
  - scope is optional — a short lowercase noun for the affected area (module, file, or feature). Related scopes may be comma-separated, e.g. \`fix(ai,settings):\`. Drop the parentheses entirely when no single area dominates.
  - description is in the imperative mood ("add", not "added"/"adds"), lowercase first letter, no trailing period.
  - Keep the whole subject line at or under 72 characters; aim for 50.
- If the change is non-trivial, add a blank line, then a body:
  - Explain WHAT changed and WHY (the motivation, the prior behavior, the problem being solved) — not HOW, which the diff already shows.
  - Name the concrete symbols, functions, or files involved where it aids understanding.
  - Write prose in complete sentences, wrapped at ~72 characters.
  - Omit the body only when the subject is fully self-explanatory (version bump, pure formatting, trivial rename).
- Breaking changes: put \`!\` after the type/scope (e.g. \`feat(api)!:\`) AND add a \`BREAKING CHANGE: <what breaks + migration>\` footer.

TYPE SELECTION
- feat: a user-facing capability is added; fix: a user-facing bug is corrected
- perf: change whose only purpose is performance; refactor: behavior-preserving restructuring
- style: formatting/whitespace only; chore: tooling, deps, versioning, housekeeping
- docs / test / build / ci: as named

RULES
- Base the message ONLY on what the diff shows. Do not invent motivation, issue numbers, or effects you cannot verify from the diff.
- If the diff is truncated, summarize what is visible without guessing at the rest.
- When recent commit subjects are provided, match their terminology and phrasing style.
- Output the raw commit message only — no surrounding quotes, no markdown fences, no preamble, no commentary.`;

export function useAiCommitMessage(repoRoot: string | null, sessionId?: string) {
  const [isGenerating, setIsGenerating] = useState(false);
  const selectedModelId = useChatStore((s) => s.selectedModelId);
  const instances = useProvidersStore((s) => s.instances);
  const instanceKeys = useProvidersStore((s) => s.instanceKeys);

  async function generate(): Promise<string | null> {
    if (!repoRoot) return null;
    setIsGenerating(true);
    try {
      // 1. Get staged diff, fall back to unstaged if nothing staged
      let diff = "";
      try {
        diff = await git.getDiff(repoRoot, ".", true, undefined, sessionId);
      } catch {
        // ignore
      }
      if (!diff.trim()) {
        try {
          diff = await git.getDiff(repoRoot, ".", false, undefined, sessionId);
        } catch {
          // ignore
        }
      }
      if (!diff.trim()) return null;

      // 2. Gather side context: the changed-file list (survives diff
      // truncation) and a few recent commit subjects as a style anchor, so
      // generated messages match this repo's conventions (scopes, phrasing).
      let fileList = "";
      try {
        const stats = await git.getDiffStats(repoRoot, sessionId);
        const lines = stats.filter((s) => s.staged).map((s) => `- ${s.path} (+${s.added} -${s.removed})`);
        const pool = lines.length > 0 ? lines : stats.map((s) => `- ${s.path} (+${s.added} -${s.removed})`);
        if (pool.length > 0) fileList = `Changed files:\n${pool.join("\n")}\n\n`;
      } catch {
        // stats unavailable — the diff still carries file headers
      }

      let styleRef = "";
      try {
        const recent = await git.getLog(repoRoot, 10, false, sessionId, undefined);
        const subjects = recent.map((c) => c.subject?.trim()).filter((s): s is string => !!s);
        if (subjects.length > 0) {
          styleRef = `Recent commit subjects in this repository (style reference only):\n${subjects
            .map((s) => `- ${s}`)
            .join("\n")}\n\n`;
        }
      } catch {
        // no history yet, or log unavailable — proceed without a style anchor
      }

      // 3. Reuse whatever model the user currently has selected in the AI
      // chat panel — same resolution path the chat agent itself uses, so the
      // commit-message generator never silently picks a different provider.
      if (instances.length === 0) {
        throw new Error("No AI provider configured. Add one in Settings → AI.");
      }
      const model = await buildModel(selectedModelId, EMPTY_PROVIDER_KEYS, {}, instances, instanceKeys);

      const { text } = await generateText({
        model,
        system: COMMIT_MSG_SYSTEM_PROMPT,
        prompt: `${styleRef}${fileList}Unified diff of the staged changes:\n\n${diff.slice(0, 16_000)}`,
      });

      return text.trim();
    } finally {
      setIsGenerating(false);
    }
  }

  return { generate, isGenerating };
}
