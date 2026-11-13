/** Static Laravel navigation through PHPantom; no project commands are executed. */
import { create } from "zustand";
import { tx } from "@/platform/i18n";
import type { Location, ProjectInfo, LaravelRoutes, LaravelViews } from "@/bindings";
import { commands, nextRequestId, isCancelled, onIdeEvent } from "@/platform/ipc";
import { useProject, relativePath } from "@/platform/project";
import { caretContext, hint, lspRequest, openLocation, reportError } from "./ipc";

export type LaravelNavigationKind = "routes" | "views";
export interface LaravelNavigationItem {
  label: string;
  name: string;
  location?: Location;
}
interface NavigationState {
  kind: LaravelNavigationKind | null;
  phase: "names" | "locations";
  selectedName: string | null;
  items: LaravelNavigationItem[];
  loading: boolean;
  incomplete: boolean;
  error: string | null;
}
const initial: NavigationState = {
  kind: null,
  phase: "names",
  selectedName: null,
  items: [],
  loading: false,
  incomplete: false,
  error: null,
};
export const useLaravelNavigation = create<NavigationState>(() => initial);
let context: { doc: number; project: ProjectInfo } | null = null;
let generation = 0;
let pending: number | null = null;

export function closeLaravelNavigation() {
  generation++;
  if (pending !== null) void commands.cancelRequest(pending).catch(() => {});
  pending = null;
  context = null;
  useLaravelNavigation.setState(initial);
}

/** Retire displayed rows and pending replies without losing the chosen logical name. */
function invalidateLaravelNavigation(message: string) {
  if (!context) return;
  generation++;
  if (pending !== null) void commands.cancelRequest(pending).catch(() => {});
  pending = null;
  useLaravelNavigation.setState({ items: [], loading: false, error: message });
}

export function installLaravelNavigationEvents() {
  const offProject = useProject.subscribe((state, previous) => {
    if (state.project !== previous.project || state.opening) closeLaravelNavigation();
  });
  const offFiles = onIdeEvent("fsChanged", ({ created, modified, removed }) => {
    if (!created.length && !modified.length && !removed.length) return;
    invalidateLaravelNavigation(tx("Project files changed. Retry to refresh Laravel results."));
  });
  const offServer = onIdeEvent("languageServer", ({ name, status }) => {
    // Initial startup and ordinary request/indexing progress must not cancel
    // the query that started the server. Restart emits stopped before starting.
    if (name !== "phpantom" || status === "starting" || status === "ready" || status === "busy") return;
    invalidateLaravelNavigation(tx("PHP server is unavailable or restarting. Retry to refresh Laravel results."));
  });
  return () => {
    offProject();
    offFiles();
    offServer();
  };
}

export async function openLaravelNavigation(kind: LaravelNavigationKind) {
  closeLaravelNavigation();
  const caret = caretContext();
  const project = useProject.getState().project;
  if (!caret || !project || !isNavigationSource(caret.path, project.root)) {
    hint(tx("Open a PHP source or Laravel translation JSON file to browse routes and views"));
    return;
  }
  context = { doc: caret.doc, project };
  useLaravelNavigation.setState({ kind });
  await query(null);
}

function isNavigationSource(path: string, root: string) {
  const file = path.replaceAll("\\", "/");
  if (file.toLowerCase().endsWith(".php")) return true;
  const prefix = root.replaceAll("\\", "/").replace(/\/+$/, "") + "/";
  if (!file.startsWith(prefix)) return false;
  const relative = file.slice(prefix.length);
  // This is an entry-point candidate; the backend validates the nearest
  // Composer root, Laravel dependency and exact translation-directory layout.
  return (
    !relative.split("/").some((part) => part === "." || part === "..") && /(?:^|\/)lang\/[^/]+\.json$/.test(relative)
  );
}

async function query(name: string | null) {
  const current = context;
  const kind = useLaravelNavigation.getState().kind;
  if (!current || !kind) return;
  const run = ++generation;
  if (pending !== null) void commands.cancelRequest(pending).catch(() => {});
  const request = nextRequestId();
  pending = request;
  useLaravelNavigation.setState({
    loading: true,
    error: null,
    items: [],
    phase: name === null ? "names" : "locations",
    selectedName: name,
  });
  try {
    const result = await lspRequest<LaravelRoutes | LaravelViews>(() =>
      kind === "routes"
        ? commands.lspLaravelRoutes(request, current.doc, name, 2000)
        : commands.lspLaravelViews(request, current.doc, name, 2000),
    );
    if (run !== generation || useProject.getState().project !== current.project) return;
    const items: LaravelNavigationItem[] =
      name !== null
        ? result.locations.map((location) => ({
            name,
            location,
            label: `${relativePath(current.project.root, location.path)}:${location.range.start.line + 1}:${location.range.start.col + 1}`,
          }))
        : "routes" in result
          ? result.routes.map((route) => {
              const label = [
                route.name,
                route.uri || tx("(unknown URI)"),
                ...(route.fromVendor ? [tx("(package)")] : []),
              ].join("  ");
              return { name: route.name, label };
            })
          : result.views.map((view) => ({ name: view, label: view }));
    useLaravelNavigation.setState({
      items,
      phase: name === null ? "names" : "locations",
      selectedName: name,
      // A complete selected-name reply cannot erase an incomplete list warning.
      incomplete: useLaravelNavigation.getState().incomplete || result.incomplete,
      loading: false,
    });
  } catch (error) {
    if (run !== generation) return;
    useLaravelNavigation.setState({
      loading: false,
      error: isCancelled(error) ? tx("Cancelled") : error instanceof Error ? error.message : String(error),
    });
  } finally {
    if (pending === request) pending = null;
  }
}

/** Refresh names in this chooser, cancelling any target lookup still in flight. */
export async function backLaravelNavigation() {
  if (!context || useLaravelNavigation.getState().phase !== "locations") return;
  if (useProject.getState().project !== context.project) {
    closeLaravelNavigation();
    return;
  }
  await query(null);
}

/** Explicitly retry the failed list/target query; never fall back to a different name. */
export async function retryLaravelNavigation() {
  const state = useLaravelNavigation.getState();
  if (!context || !state.kind || !state.error || state.loading) return;
  if (useProject.getState().project !== context.project) {
    closeLaravelNavigation();
    return;
  }
  await query(state.selectedName);
}

export async function selectLaravelItem(item: LaravelNavigationItem) {
  const state = useLaravelNavigation.getState();
  if (!context || state.loading) return;
  if (useProject.getState().project !== context.project) {
    closeLaravelNavigation();
    return;
  }
  // A queued activation must not reuse rows from a replaced query or project.
  if (!state.items.includes(item)) return;
  if (!item.location) {
    await query(item.name);
    return;
  }
  const location = item.location;
  closeLaravelNavigation();
  try {
    await openLocation(location.uri ?? location.path, location.range.start.line, location.range.start.col);
  } catch (error) {
    reportError(error);
  }
}
