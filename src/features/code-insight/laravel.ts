/** Static Laravel navigation through PHPantom; no project commands are executed. */
import { create } from "zustand";
import { activeEditor, onEditorEvent, whenAllSynced } from "@/editor/api";
import { tx } from "@/platform/i18n";
import type { Location, ProjectInfo, LaravelRoutes, LaravelViews, LaravelRouteHandler } from "@/bindings";
import { commands, nextRequestId, isCancelled, onIdeEvent } from "@/platform/ipc";
import { useProject, relativePath } from "@/platform/project";
import { validateProjectStructure } from "@/platform/languageSettings";
import { isVirtualUri } from "@/platform/virtualDocs";
import { caretContext, hint, lspRequest, openLocation, reportError, type CaretContext } from "./ipc";

export type LaravelNavigationKind = "routes" | "views" | "handler";
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
function handlerFrame(caret: CaretContext) {
  return {
    pos: { line: caret.line, col: caret.col },
    path: caret.path,
    label: caret.word,
    text: caret.view?.state.doc ?? null,
    view: caret.view,
    selection: caret.view?.state.selection ?? null,
  };
}
type NavigationContext = { doc: number; project: ProjectInfo; handler?: ReturnType<typeof handlerFrame> };
let context: NavigationContext | null = null;
let generation = 0;
let pending: number | null = null;
let syncing: number | null = null;

function isHandlerSource(caret: CaretContext) {
  const editor = activeEditor();
  return caret.doc >= 0 && editor?.ctx.doc === caret.doc && editor.ctx.language === "php" && !isVirtualUri(caret.path);
}

function handlerSourceCurrent(current: NavigationContext) {
  const frame = current.handler;
  const editor = activeEditor();
  return (
    !frame ||
    (editor?.ctx.doc === current.doc &&
      editor.ctx.language === "php" &&
      editor.ctx.path === frame.path &&
      (frame.text === null || editor.view.state.doc === frame.text))
  );
}

function handlerPositionCurrent(current: NavigationContext) {
  const frame = current.handler;
  if (!frame) return true;
  if (frame.view && frame.selection)
    return activeEditor()?.view === frame.view && frame.view.state.selection.eq(frame.selection);
  const caret = caretContext();
  return (
    caret?.doc === current.doc &&
    caret.path === frame.path &&
    caret.line === frame.pos.line &&
    caret.col === frame.pos.col
  );
}

function handlerInputCurrent(current: NavigationContext) {
  if (current.handler?.view && activeEditor()?.view !== current.handler.view) {
    closeLaravelNavigation();
    return false;
  }
  if (!handlerSourceCurrent(current)) {
    invalidateLaravelNavigation(tx("Project files changed. Retry to refresh Laravel results."));
    return false;
  }
  if (!handlerPositionCurrent(current)) {
    closeLaravelNavigation();
    return false;
  }
  return true;
}

export function closeLaravelNavigation() {
  generation++;
  if (pending !== null) void commands.cancelRequest(pending).catch(() => {});
  pending = null;
  syncing = null;
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
    const roots = (value: typeof state) => (value.project ? navigationRoots(value.project, value.settings) : []);
    if (
      state.project !== previous.project ||
      state.opening ||
      JSON.stringify(roots(state)) !== JSON.stringify(roots(previous))
    )
      closeLaravelNavigation();
  });
  const changed = () => invalidateLaravelNavigation(tx("Project files changed. Retry to refresh Laravel results."));
  const offEditors = onEditorEvent((event) => {
    if (event.type === "docChanged") {
      // These acknowledgements are included in the query's pending-edit barrier.
      if (
        syncing !== generation ||
        (context?.handler &&
          event.doc === context.doc &&
          (context.handler.text === null || !handlerSourceCurrent(context)))
      )
        changed();
    } else if (event.type === "docClosed") {
      if (event.doc === context?.doc) closeLaravelNavigation();
      else changed();
    } else if (event.type === "activeEditorChanged" && context?.handler) {
      if (event.doc !== context.doc || !handlerPositionCurrent(context)) closeLaravelNavigation();
    } else if (
      event.type === "caretMoved" &&
      context?.handler &&
      event.doc === context.doc &&
      // Text edits retain the existing Retry state even when they map the caret.
      handlerSourceCurrent(context) &&
      !handlerPositionCurrent(context)
    )
      closeLaravelNavigation();
  });
  const offPatches = onIdeEvent("docPatched", changed);
  const offMoves = onIdeEvent("docMoved", ({ doc }) => {
    if (doc === context?.doc) closeLaravelNavigation();
    else changed();
  });
  const offFiles = onIdeEvent("fsChanged", ({ created, modified, removed }) => {
    if (!created.length && !modified.length && !removed.length) return;
    changed();
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
    offEditors();
    offPatches();
    offMoves();
    offServer();
  };
}

export async function openLaravelNavigation(kind: LaravelNavigationKind) {
  closeLaravelNavigation();
  const caret = caretContext();
  const { project, settings } = useProject.getState();
  if (
    !caret ||
    !project ||
    !(kind === "handler" ? isHandlerSource(caret) : isNavigationSource(caret.path, navigationRoots(project, settings)))
  ) {
    hint(
      kind === "handler"
        ? tx("Open a PHP source to find its invokable route handler")
        : tx("Open a PHP source or Laravel translation JSON file to browse routes and views"),
    );
    return;
  }
  context = { doc: caret.doc, project, ...(kind === "handler" ? { handler: handlerFrame(caret) } : {}) };
  useLaravelNavigation.setState({ kind });
  await query(null);
}

function navigationRoots(project: ProjectInfo, settings: ProjectInfo["settings"] | null) {
  const extra = project.workspace
    ? []
    : validateProjectStructure((settings ?? project.settings).projectStructure).contentRoots.map((root) => root.trim());
  return [...new Set([project.root, ...extra].map((root) => root.replaceAll("\\", "/").replace(/\/+$/, "")))].sort();
}

function isNavigationSource(path: string, roots: string[]) {
  const file = path.replaceAll("\\", "/");
  if (file.toLowerCase().endsWith(".php")) return true;
  // Candidate only: the backend validates the current root, Composer manifest,
  // Laravel dependency and exact translation-directory layout.
  return (
    !file.split("/").some((part) => part === "." || part === "..") &&
    /(?:^|\/)lang\/[^/]+\.json$/.test(file) &&
    roots.some((root) => file.startsWith(root + "/"))
  );
}

async function query(name: string | null) {
  const current = context;
  const kind = useLaravelNavigation.getState().kind;
  if (!current || !kind || (kind === "handler" && !current.handler)) return;
  const phase = kind === "handler" || name !== null ? "locations" : "names";
  const selectedName = kind === "handler" ? current.handler!.label : name;
  const run = ++generation;
  if (pending !== null) void commands.cancelRequest(pending).catch(() => {});
  const request = nextRequestId();
  pending = null;
  syncing = run;
  useLaravelNavigation.setState({
    loading: true,
    error: null,
    items: [],
    phase,
    selectedName,
  });
  try {
    await whenAllSynced();
    if (syncing === run) syncing = null;
    if (run !== generation || useProject.getState().project !== current.project) return;
    if (!handlerInputCurrent(current)) return;
    pending = request;
    const result = await lspRequest<LaravelRoutes | LaravelViews | LaravelRouteHandler>(() =>
      kind === "handler"
        ? commands.lspLaravelRouteHandler(request, current.doc, current.handler!.pos)
        : kind === "routes"
          ? commands.lspLaravelRoutes(request, current.doc, name, 2000)
          : commands.lspLaravelViews(request, current.doc, name, 2000),
    );
    if (run !== generation || useProject.getState().project !== current.project) return;
    if (pending === request) pending = null;
    if (!handlerInputCurrent(current)) return;
    const items: LaravelNavigationItem[] =
      kind === "handler" || name !== null
        ? result.locations.map((location) => ({
            name: selectedName ?? "",
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
          : "views" in result
            ? result.views.map((view) => ({ name: view, label: view }))
            : [];
    const incomplete = useLaravelNavigation.getState().incomplete || result.incomplete;
    useLaravelNavigation.setState({
      items,
      phase,
      selectedName,
      // A complete selected-name reply cannot erase an incomplete list warning.
      incomplete,
      loading: false,
    });
    if (kind === "handler" && !incomplete && items.length === 1) await selectLaravelItem(items[0]!);
  } catch (error) {
    if (pending === request) pending = null;
    if (run !== generation || !handlerInputCurrent(current)) return;
    useLaravelNavigation.setState({
      loading: false,
      error: isCancelled(error) ? tx("Cancelled") : error instanceof Error ? error.message : String(error),
    });
  } finally {
    if (syncing === run) syncing = null;
    if (pending === request) pending = null;
  }
}

/** Refresh names in this chooser, cancelling any target lookup still in flight. */
export async function backLaravelNavigation() {
  if (
    !context ||
    useLaravelNavigation.getState().kind === "handler" ||
    useLaravelNavigation.getState().phase !== "locations"
  )
    return;
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
  if (state.kind === "handler") {
    const caret = caretContext();
    if (!caret || caret.doc !== context.doc || !isHandlerSource(caret)) {
      hint(tx("Return to the original PHP source to retry handler lookup"));
      return;
    }
    context.handler = handlerFrame(caret);
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
  const current = context;
  const run = generation;
  useLaravelNavigation.setState({ loading: true });
  try {
    await whenAllSynced();
  } catch (error) {
    if (context === current && run === generation && handlerInputCurrent(current))
      invalidateLaravelNavigation(error instanceof Error ? error.message : String(error));
    return;
  }
  if (
    context !== current ||
    run !== generation ||
    useProject.getState().project !== current.project ||
    !useLaravelNavigation.getState().items.includes(item)
  )
    return;
  if (!handlerInputCurrent(current)) return;
  const location = item.location;
  closeLaravelNavigation();
  try {
    await openLocation(location.uri ?? location.path, location.range.start.line, location.range.start.col);
  } catch (error) {
    reportError(error);
  }
}
