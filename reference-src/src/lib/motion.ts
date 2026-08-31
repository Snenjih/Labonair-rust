/**
 * Shared `motion/react` transition constants, mirroring the CSS
 * `--dur-*`/`--ease-*` tokens in globals.css so custom app components and
 * shadcn/Radix primitives feel like one system instead of ad-hoc values.
 */

export const DURATION = {
  fast: 0.16, // --dur-fast: 160ms
  base: 0.24, // --dur-base: 240ms
  slow: 0.32, // --dur-slow: 320ms
} as const;

export const EASE_PREMIUM = [0.16, 1, 0.3, 1] as const; // --ease-premium
export const EASE_SOFT = [0.4, 0, 0.2, 1] as const; // --ease-soft

export const SPRING_SNAPPY = { type: "spring", stiffness: 400, damping: 36 } as const;
export const SPRING_SOFT = { type: "spring", stiffness: 300, damping: 32 } as const;
