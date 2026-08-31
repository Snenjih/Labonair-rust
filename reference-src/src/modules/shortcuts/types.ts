export type KeyBinding = {
  key: string;
  /** Physical key from `e.code` (e.g. "KeyK", "Digit1", "Comma") — used to
   *  build native-menu accelerator strings (see lib/nativeMenuSync.ts)
   *  without the ambiguity `key` alone has for shifted characters like "?"
   *  or "+". Bindings saved before this field existed won't have it at
   *  runtime despite the type being required here — only the native-menu
   *  sync path needs to defend against that (see buildMenuSyncPayload). */
  code: string;
  meta: boolean;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  displayKeys: string[];
};

export type KeyBindingOrDisabled = KeyBinding | null;
export type KeyBindingMap = Partial<Record<string, KeyBindingOrDisabled>>;
