/**
 * Reload Environment: new terminals, runs, language servers and tools get the login
 * shell's current environment (and, on a remote host, the project's `devcontainer.json`
 * `remoteEnv`) without restarting the backend. What is already running keeps its own.
 */
import { tx } from "@/platform/i18n";
import { executeAction } from "@/platform/actions";
import { call, commands, type EnvReloaded } from "@/platform/ipc";
import { notifications } from "@/workbench/contributions";

const KEY = "remote.reloadEnvironment";
/** Variable names listed in the notification before "and N more". */
const LISTED = 8;

/** The notification text for a reload result. */
export function reloadSummary(r: EnvReloaded): string {
  const names = r.changed.slice(0, LISTED).join(", ");
  const more = r.changed.length - LISTED;
  const changed =
    r.changed.length === 0
      ? tx("No variables changed.")
      : more > 0
        ? tx`Changed: ${names} and ${more} more.`
        : tx`Changed: ${names}.`;
  const keep = tx("Terminals and processes already running keep their old environment; new ones get the new one.");
  return [changed, keep].join(" ");
}

export async function reloadEnvironment() {
  let result: EnvReloaded;
  try {
    result = await call(commands.envReload());
  } catch (e) {
    notifications().notify({
      key: KEY,
      level: "error",
      title: "Could not reload the environment",
      body: e instanceof Error ? e.message : String(e),
    });
    return;
  }
  notifications().notify({
    key: KEY,
    level: "info",
    title: "Environment reloaded",
    body: reloadSummary(result),
    actions: result.changed.length
      ? [{ label: "Restart Language Servers", run: () => void executeAction("code.restartLanguageServers") }]
      : [],
  });
}
