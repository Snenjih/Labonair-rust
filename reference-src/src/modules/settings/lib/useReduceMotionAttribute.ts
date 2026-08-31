import { useEffect } from "react";

/**
 * Mirrors `reduceMotion` onto a `data-reduce-motion` attribute on <html> so
 * the CSS-level reduce-motion reset in globals.css (which covers Tailwind/
 * Radix animations that <MotionConfig> can't reach) can target it. Call once
 * per window (main window + settings window each have their own <html>).
 */
export function useReduceMotionAttribute(reduceMotion: boolean): void {
  useEffect(() => {
    document.documentElement.setAttribute("data-reduce-motion", reduceMotion ? "true" : "false");
  }, [reduceMotion]);
}
