import { useEffect } from "react";
import { usePreferencesStore } from "@/modules/settings/preferences";

export function useLayoutEngine(): void {
  const radius = usePreferencesStore((s) => s.appCornerRadius);

  useEffect(() => {
    document.documentElement.style.setProperty("--radius", `${radius}px`);
  }, [radius]);
}
