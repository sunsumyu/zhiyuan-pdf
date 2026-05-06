type TauriInternals = {
  invoke?: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
};

export type PlatformWindow = Window &
  Record<string, any> & {
  __TAURI_INTERNALS__?: TauriInternals;
  webkitAudioContext?: typeof AudioContext;
};

export function getPlatformWindow(): PlatformWindow {
  return window as PlatformWindow;
}

export function getTauriInternals(): TauriInternals | undefined {
  return getPlatformWindow().__TAURI_INTERNALS__;
}

export function isTauriRuntime(): boolean {
  return Boolean(getTauriInternals());
}
