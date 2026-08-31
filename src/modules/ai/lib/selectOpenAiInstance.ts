import type { ProviderInstance } from "../config";

/** Picks the API key to use for Whisper transcription: the first configured
 *  real-`openai`-provider instance (not the broader OpenAI-compatible/local
 *  set — `experimental_transcribe` + `openai.transcription("whisper-1")`
 *  assumes OpenAI's actual API shape) that has a key. Returns `null` if none
 *  is configured, in which case voice input stays disabled — same UX as
 *  before this was migrated off the legacy per-provider key store. */
export function selectOpenAiInstanceKey(
  instances: ProviderInstance[],
  instanceKeys: Record<string, string | null>,
): string | null {
  const openaiInstance = instances.find((i) => i.providerId === "openai" && !!instanceKeys[i.id]);
  return openaiInstance ? (instanceKeys[openaiInstance.id] ?? null) : null;
}
