// The only module that knows command and event names.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BackendError, Provider, RelatedProcess, Snapshot, UpdateInfo } from "./types";

export const ipc = {
  snapshot: () => invoke<Snapshot>("get_snapshot"),
  refresh: (force: boolean) => invoke<void>("refresh", { force }),
  addAccount: (provider: Provider) => invoke<string>("add_account", { provider }),
  importAccount: (provider: Provider, expectedId?: string) =>
    invoke<string>("import_account", { provider, expectedId }),
  cancelAddAccount: () => invoke<void>("cancel_add_account"),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  switchAccount: (id: string) => invoke<void>("switch_account", { id }),
  runningProcesses: (provider: Provider) => invoke<RelatedProcess[]>("running_processes", { provider }),
  stopClient: (provider: Provider) => invoke<void>("stop_client", { provider }),
  restartClient: (provider: Provider) => invoke<void>("restart_client", { provider }),
  resizePanel: (height: number) => invoke<void>("resize_panel", { height }),
  revealLogs: () => invoke<void>("reveal_logs"),
  autostartEnabled: () => invoke<boolean>("autostart_enabled"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: (version: string) => invoke<void>("install_update", { version }),
  quit: () => invoke<void>("quit"),
};

export const onSnapshot = (handler: (snapshot: Snapshot) => void): Promise<UnlistenFn> =>
  listen<Snapshot>("orbitswitch://snapshot", (event) => handler(event.payload));

export const onShown = (handler: () => void): Promise<UnlistenFn> => listen("orbitswitch://shown", handler);

export function asBackendError(error: unknown): BackendError {
  if (error && typeof error === "object" && "code" in error && "message" in error) {
    return error as BackendError;
  }
  return { code: "unexpected", message: String(error) };
}
