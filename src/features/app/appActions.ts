import { editCommand } from "./editCommand";
export { editCommand } from "./editCommand";
import type { Action } from "@/platform/actions";
import { revealInFileManager } from "@/platform/fileManager";
import { t } from "@/platform/i18n";
import { call, commands, isTauri } from "@/platform/ipc";
import { notifications } from "@/workbench/contributions";
import { showAbout } from "./AboutDialog";
import { quitApp } from "./closeGuard";
import { guessOs } from "./host";
import { showKeymapReference } from "./KeymapReference";
import { checkForUpdates } from "./updates";

function errorText(e: unknown) {
  return e instanceof Error ? e.message : String(e);
}

export async function showLog(): Promise<void> {
  if (!isTauri()) return;
  try {
    const loc = await call(commands.appLogLocation());
    await revealInFileManager(loc.file ?? loc.dir, null);
  } catch (e) {
    notifications().notify({ level: "error", title: t("app.log.failed"), body: errorText(e) });
  }
}

/** Registered only when the build has an update endpoint (`BuildInfo.updates`). */
export function updateActions(): Action[] {
  return [
    {
      id: "app.checkForUpdates",
      title: t("app.action.checkForUpdates"),
      category: t("app.category.help"),
      keywords: ["update", "upgrade", "version"],
      run: checkForUpdates,
    },
  ];
}

export function appActions(): Action[] {
  const os = guessOs();
  const edit = (id: string, command: Parameters<typeof editCommand>[0], key: string): Action => ({
    id,
    title: t(key),
    category: t("app.category.edit"),
    hidden: true,
    run: editCommand(command),
  });
  return [
    { id: "app.about", title: t("app.action.about"), category: t("app.category.help"), icon: "info", run: showAbout },
    { id: "app.quit", title: t("app.action.quit", { os }), category: t("app.category.file"), run: () => quitApp() },
    {
      id: "help.keymapReference",
      title: t("app.action.keymapReference"),
      category: t("app.category.help"),
      icon: "keyboard",
      keywords: ["shortcuts", "keyboard", "keymap", "cheat sheet"],
      run: showKeymapReference,
    },
    {
      id: "help.showLog",
      title: t("app.action.showLog", { os }),
      category: t("app.category.help"),
      keywords: ["log", "logs", "openidea.log"],
      run: showLog,
    },
    edit("edit.undo", "undo", "app.action.undo"),
    edit("edit.redo", "redo", "app.action.redo"),
    edit("edit.cut", "cut", "app.action.cut"),
    edit("edit.copy", "copy", "app.action.copy"),
    edit("edit.paste", "paste", "app.action.paste"),
    edit("edit.selectAll", "selectAll", "app.action.selectAll"),
  ];
}
