// Native context menus, so they look and behave like the platform's own.
//
// Every item is created as its own resource (MenuItem.new and friends)
// instead of inline in Menu.new: Tauri 2.12 drops inline items as soon as the
// menu is built, and dropping an item removes its click handler, so inline
// items never respond. The resources behind the menu on screen are released
// when the next menu opens.

import type { Resource } from "@tauri-apps/api/core";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MouseEvent } from "react";
import { t } from "./i18n";
import { ipc } from "./ipc";
import type { Account } from "./types";

const REPOSITORY = "https://github.com/shengyy/orbitswitch";
const WEBSITE = "https://shengyy.github.io/orbitswitch/";

type Entry = "separator" | { text: string; action?: () => void; enabled?: boolean; checked?: boolean };

let onScreen: Resource[] = [];

function anchor(event: MouseEvent): LogicalPosition {
  const target = event.currentTarget as HTMLElement | null;
  if (event.type === "click" && target) {
    const rect = target.getBoundingClientRect();
    return new LogicalPosition(rect.left, rect.bottom + 4);
  }
  return new LogicalPosition(event.clientX, event.clientY);
}

async function popup(entries: Entry[], at: LogicalPosition) {
  for (const resource of onScreen) void resource.close();
  const items = await Promise.all(
    entries.map((entry) => {
      if (entry === "separator") return PredefinedMenuItem.new({ item: "Separator" });
      const { text, action, enabled = true, checked } = entry;
      return checked === undefined
        ? MenuItem.new({ text, enabled, action })
        : CheckMenuItem.new({ text, enabled, checked, action });
    }),
  );
  const menu = await Menu.new({ items });
  onScreen = [menu, ...items];
  await menu.popup(at);
}

export async function showAppMenu(
  event: MouseEvent,
  version: string,
  update: { checking: boolean; enabled: boolean; onCheck: () => void },
) {
  const at = anchor(event);
  const launchAtLogin = await ipc.autostartEnabled().catch(() => false);
  await popup(
    [
      { text: `OrbitSwitch ${version}`, enabled: false },
      {
        text: t(update.checking ? "update.checking" : "menu.checkUpdate"),
        enabled: update.enabled && !update.checking,
        action: update.onCheck,
      },
      "separator",
      {
        text: t("menu.launchAtLogin"),
        checked: launchAtLogin,
        action: () => void ipc.setAutostart(!launchAtLogin),
      },
      { text: t("menu.logs"), action: () => void ipc.revealLogs() },
      "separator",
      { text: t("menu.website"), action: () => void openUrl(WEBSITE) },
      { text: t("menu.github"), action: () => void openUrl(REPOSITORY) },
      "separator",
      { text: t("menu.quit"), action: () => void ipc.quit() },
    ],
    at,
  );
}

interface AccountActions {
  onReauth: () => void;
  onRemove: () => void;
}

export async function showAccountMenu(event: MouseEvent, account: Account, actions: AccountActions) {
  await popup(
    [
      { text: t("menu.reauth"), action: actions.onReauth },
      // A native menu click is not a page gesture, so the web clipboard API would refuse it.
      { text: t("menu.copyEmail"), action: () => void writeText(account.email) },
      "separator",
      { text: t("menu.remove"), action: actions.onRemove },
    ],
    anchor(event),
  );
}
