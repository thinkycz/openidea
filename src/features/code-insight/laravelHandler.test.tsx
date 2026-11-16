import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { EditorView } from "@codemirror/view";
import { EditorSelection, EditorState } from "@codemirror/state";
import type { ProjectInfo } from "@/bindings";

const mock = vi.hoisted(() => ({
  handler: vi.fn(),
  routes: vi.fn(),
  views: vi.fn(),
  cancel: vi.fn(),
  open: vi.fn(),
  hint: vi.fn(),
  caret: vi.fn(),
  language: "php",
}));
vi.mock("@/platform/ipc", async (orig) => {
  const actual = await orig<typeof import("@/platform/ipc")>();
  return {
    ...actual,
    isTauri: () => true,
    commands: {
      ...actual.commands,
      lspLaravelRouteHandler: mock.handler,
      lspLaravelRoutes: mock.routes,
      lspLaravelViews: mock.views,
      cancelRequest: mock.cancel,
    },
  };
});
vi.mock("@/editor/api", async (orig) => {
  const actual = await orig<typeof import("@/editor/api")>();
  return {
    ...actual,
    activeEditor: () => ({
      ctx: { doc: mock.caret()?.doc, path: mock.caret()?.path, language: mock.language },
      view: mock.caret()?.view,
    }),
  };
});
vi.mock("./ipc", async (orig) => ({
  ...(await orig<typeof import("./ipc")>()),
  caretContext: mock.caret,
  openLocation: mock.open,
  hint: mock.hint,
}));
const { useProject } = await import("@/platform/project");
const {
  openLaravelNavigation,
  closeLaravelNavigation,
  installLaravelNavigationEvents,
  retryLaravelNavigation,
  selectLaravelItem,
  useLaravelNavigation,
} = await import("./laravel");
const { LaravelNavigation } = await import("./LaravelNavigation");
const { _emitEditorEvent, registerPendingEdits } = await import("@/editor/api");
const { emitLocalIdeEvent } = await import("@/platform/ipc");
const target = { path: "/p/app/Action.php", range: { start: { line: 3, col: 14 }, end: { line: 3, col: 22 } } };
const ok = (incomplete = false, locations = [target]) => ({ status: "ok", data: { locations, incomplete } });
let off: () => void;
let view: EditorView | null = null;
beforeEach(() => {
  vi.clearAllMocks();
  mock.language = "php";
  mock.cancel.mockResolvedValue(undefined);
  mock.open.mockResolvedValue(undefined);
  mock.caret.mockReturnValue({ doc: 7, path: "/p/routes/web.php", line: 4, col: 26, word: "Action", view: null });
  mock.handler.mockResolvedValue(ok());
  mock.routes.mockResolvedValue({ status: "ok", data: { routes: [], locations: [], incomplete: false } });
  mock.views.mockResolvedValue({ status: "ok", data: { views: [], locations: [], incomplete: false } });
  useProject.setState({
    project: { root: "/p", name: "p", isGit: false, settings: {} } as ProjectInfo,
    opening: false,
    settings: null,
  });
  off = installLaravelNavigationEvents();
});
afterEach(() => {
  off();
  act(() => closeLaravelNavigation());
  const parent = view?.dom.parentElement;
  view?.destroy();
  parent?.remove();
  view = null;
});
const changed = (doc = 7) =>
  _emitEditorEvent({
    type: "docChanged",
    doc,
    path: doc === 7 ? "/p/routes/web.php" : "/p/app/Action.php",
    baseVersion: 1,
    version: 2,
    changes: [{ from: 0, to: 0, insert: " " }],
  });

it("opens a single complete invokable implementation using the original UTF16 caret", async () => {
  await openLaravelNavigation("handler");
  expect(mock.handler).toHaveBeenCalledExactlyOnceWith(expect.any(Number), 7, { line: 4, col: 26 });
  expect(mock.open).toHaveBeenCalledExactlyOnceWith(target.path, 3, 14);
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(mock.routes).not.toHaveBeenCalled();
  expect(mock.views).not.toHaveBeenCalled();
  expect(mock.cancel).not.toHaveBeenCalled();
});
it("requires selection for an incomplete single handler and has no names Back button", async () => {
  mock.handler.mockResolvedValue(ok(true));
  await openLaravelNavigation("handler");
  render(<LaravelNavigation />);
  expect(screen.getByRole("dialog", { name: "Invokable Route Handlers" })).toBeVisible();
  expect(screen.getByText(/Results are incomplete/)).toBeVisible();
  expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
  expect(mock.open).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Open" }));
  await waitFor(() => expect(mock.open).toHaveBeenCalledWith(target.path, 3, 14));
});
it("preserves duplicate complete targets for explicit selection", async () => {
  mock.handler.mockResolvedValue(ok(false, [target, target]));
  await openLaravelNavigation("handler");
  expect(useLaravelNavigation.getState().items).toHaveLength(2);
  expect(mock.open).not.toHaveBeenCalled();
  await selectLaravelItem(useLaravelNavigation.getState().items[1]!);
  expect(mock.open).toHaveBeenCalledOnce();
});
it("uses the configured PHP language rather than a filename extension", async () => {
  mock.caret.mockReturnValue({ ...mock.caret(), path: "/p/routes/script.inc" });
  await openLaravelNavigation("handler");
  expect(mock.handler).toHaveBeenCalledOnce();
  mock.language = "json";
  mock.caret.mockReturnValue({ ...mock.caret(), path: "/p/lang/en.json" });
  await openLaravelNavigation("handler");
  expect(mock.handler).toHaveBeenCalledOnce();
  expect(mock.hint).toHaveBeenCalledWith("Open a PHP source to find its invokable route handler");
});
it("keeps unsupported handler lookup errors visible without probing routes or views", async () => {
  mock.handler.mockResolvedValue({
    status: "error",
    error: { kind: "unavailable", message: "phpantom doesn't support Laravel route handler", data: null },
  });
  await openLaravelNavigation("handler");
  expect(useLaravelNavigation.getState().error).toContain("doesn't support");
  expect(mock.routes).not.toHaveBeenCalled();
  expect(mock.views).not.toHaveBeenCalled();
  expect(mock.open).not.toHaveBeenCalled();
});
it.each(["edit", "project", "cancel", "move", "server"])(
  "discards a late complete handler after %s",
  async (mutation) => {
    let resolve!: (value: unknown) => void;
    mock.handler.mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const pending = openLaravelNavigation("handler");
    await waitFor(() => expect(resolve).toBeTypeOf("function"));
    if (mutation === "edit") changed();
    else if (mutation === "project") useProject.setState({ project: null });
    else if (mutation === "move")
      emitLocalIdeEvent({ type: "docMoved", doc: 7, from: "/p/routes/web.php", to: "/p/routes/other.php" });
    else if (mutation === "server")
      emitLocalIdeEvent({ type: "languageServer", server: 1, name: "phpantom", status: "stopped", message: null });
    else closeLaravelNavigation();
    resolve(ok());
    await pending;
    expect(mock.open).not.toHaveBeenCalled();
    expect(mock.cancel).toHaveBeenCalledOnce();
  },
);
it("recaptures a changed caret only in the original document on explicit Retry", async () => {
  mock.handler.mockResolvedValue(ok(true));
  await openLaravelNavigation("handler");
  changed();
  mock.caret.mockReturnValue({ ...mock.caret(), doc: 8 });
  await retryLaravelNavigation();
  expect(mock.handler).toHaveBeenCalledTimes(1);
  expect(mock.hint).toHaveBeenLastCalledWith("Return to the original PHP source to retry handler lookup");
  mock.caret.mockReturnValue({ ...mock.caret(), doc: 7, line: 6, col: 31, word: "Other" });
  await retryLaravelNavigation();
  expect(mock.handler).toHaveBeenLastCalledWith(expect.any(Number), 7, { line: 6, col: 31 });
  expect(useLaravelNavigation.getState().incomplete).toBe(true);
  expect(mock.open).not.toHaveBeenCalled();
});
it.each([false, true])("checks source text around the pending-edit barrier (changed=%s)", async (edit) => {
  const parent = document.createElement("div");
  document.body.append(parent);
  view = new EditorView({ doc: "<?php // 😀\nAction::class;\n", parent });
  mock.caret.mockReturnValue({ ...mock.caret(), view, line: 1, col: 0 });
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const dispose = registerPendingEdits(() => barrier);
  const pending = openLaravelNavigation("handler");
  expect(mock.handler).not.toHaveBeenCalled();
  if (edit) view.dispatch({ changes: { from: 0, insert: "\n" } });
  else changed();
  release();
  await pending;
  dispose();
  if (edit) {
    expect(mock.handler).not.toHaveBeenCalled();
    expect(useLaravelNavigation.getState().error).toContain("Project files changed");
  } else {
    expect(mock.handler).toHaveBeenCalledOnce();
    expect(mock.open).toHaveBeenCalledOnce();
  }
});
it("retires source edits during the barrier even without an editor view", async () => {
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const dispose = registerPendingEdits(() => barrier);
  const pending = openLaravelNavigation("handler");
  changed();
  release();
  await pending;
  dispose();
  expect(mock.handler).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().error).toContain("Project files changed");
});

it("rejects a silent source edit before a complete response can auto-open", async () => {
  const parent = document.createElement("div");
  document.body.append(parent);
  view = new EditorView({ doc: "<?php Action::class;", parent });
  mock.caret.mockReturnValue({ ...mock.caret(), view });
  let resolve!: (value: unknown) => void;
  mock.handler.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("handler");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  view.dispatch({ changes: { from: 0, insert: "\n" } });
  resolve(ok());
  await pending;
  expect(mock.open).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().error).toContain("Project files changed");
});
it("waits for pending dependency edits before opening a displayed handler", async () => {
  mock.handler.mockResolvedValue(ok(true));
  await openLaravelNavigation("handler");
  const item = useLaravelNavigation.getState().items[0]!;
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const dispose = registerPendingEdits(() => barrier);
  const selection = selectLaravelItem(item);
  expect(useLaravelNavigation.getState().loading).toBe(true);
  expect(mock.open).not.toHaveBeenCalled();
  changed(8);
  release();
  await selection;
  dispose();
  expect(mock.open).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().error).toContain("Project files changed");
});
it("does not auto-open when Retry returns complete data after an incomplete reply", async () => {
  mock.handler.mockResolvedValue(ok(true));
  await openLaravelNavigation("handler");
  changed(8);
  mock.handler.mockResolvedValue(ok());
  await retryLaravelNavigation();
  expect(useLaravelNavigation.getState().incomplete).toBe(true);
  expect(useLaravelNavigation.getState().items).toHaveLength(1);
  expect(mock.open).not.toHaveBeenCalled();
});
it("retires a pending handler when another editor becomes active", async () => {
  let resolve!: (value: unknown) => void;
  mock.handler.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("handler");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  _emitEditorEvent({ type: "activeEditorChanged", doc: 9, path: "/p/other.php" });
  resolve(ok());
  await pending;
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(mock.open).not.toHaveBeenCalled();
});

it("explains where to invoke handler lookup when the caret has no static association", async () => {
  mock.handler.mockResolvedValue(ok(false, []));
  await openLaravelNavigation("handler");
  render(<LaravelNavigation />);
  expect(screen.getByText("Place the caret on an invokable controller in a route action.")).toBeVisible();
  expect(screen.getByText("No static matches found")).toBeVisible();
  expect(screen.getByRole("button", { name: "Open" })).toBeDisabled();
  expect(mock.open).not.toHaveBeenCalled();
});

it("does not dispatch an old handler position after the pending-edit barrier", async () => {
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const dispose = registerPendingEdits(() => barrier);
  try {
    const pending = openLaravelNavigation("handler");
    mock.caret.mockReturnValue({ ...mock.caret(), col: 27 });
    release();
    await pending;
    expect(mock.handler).not.toHaveBeenCalled();
    expect(useLaravelNavigation.getState().kind).toBeNull();
    expect(mock.open).not.toHaveBeenCalled();
  } finally {
    dispose();
  }
});
it.each(["line", "col"])("quietly discards a completed handler after the source %s changed", async (coordinate) => {
  let resolve!: (value: unknown) => void;
  mock.handler.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("handler");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  mock.caret.mockReturnValue({ ...mock.caret(), [coordinate]: mock.caret()[coordinate] + 1 });
  resolve(ok());
  await pending;
  expect(mock.open).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(mock.hint).not.toHaveBeenCalled();
});
it("checks the real source selection before a late reply even when the caret cache did not update", async () => {
  const parent = document.createElement("div");
  document.body.append(parent);
  view = new EditorView({ doc: "<?php Action::class;", selection: EditorSelection.cursor(10), parent });
  mock.caret.mockReturnValue({ ...mock.caret(), view, line: 0, col: 10 });
  let resolve!: (value: unknown) => void;
  mock.handler.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("handler");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  view.dispatch({ selection: EditorSelection.range(7, 10) });
  resolve(ok());
  await pending;
  expect(mock.open).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().kind).toBeNull();
});
it.each([true, false])(
  "retires a pending handler when another pane of the same document becomes active (event=%s)",
  async (event) => {
    const parent = document.createElement("div");
    document.body.append(parent);
    view = new EditorView({ doc: "<?php Action::class;", parent });
    mock.caret.mockReturnValue({ ...mock.caret(), view });
    let resolve!: (value: unknown) => void;
    mock.handler.mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const pending = openLaravelNavigation("handler");
    await waitFor(() => expect(resolve).toBeTypeOf("function"));
    const other = new EditorView({
      state: EditorState.create({ doc: event ? view.state.doc : view.state.doc.toString() }),
    });
    try {
      mock.caret.mockReturnValue({ ...mock.caret(), view: other });
      if (event) _emitEditorEvent({ type: "activeEditorChanged", doc: 7, path: "/p/routes/web.php" });
      resolve(ok());
      await pending;
      expect(mock.open).not.toHaveBeenCalled();
      expect(useLaravelNavigation.getState().kind).toBeNull();
      expect(mock.cancel).toHaveBeenCalledTimes(event ? 1 : 0);
    } finally {
      other.destroy();
    }
  },
);
it("cancels an in-flight handler on a source caret event but ignores other documents and unchanged positions", async () => {
  let resolve!: (value: unknown) => void;
  mock.handler.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("handler");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  const event = { type: "caretMoved", doc: 7, path: "/p/routes/web.php", line: 4, col: 26, offset: 100 } as const;
  _emitEditorEvent({ ...event, doc: 8 });
  _emitEditorEvent(event);
  expect(mock.cancel).not.toHaveBeenCalled();
  mock.caret.mockReturnValue({ ...mock.caret(), col: 27 });
  _emitEditorEvent({ ...event, col: 27, offset: 101 });
  expect(mock.cancel).toHaveBeenCalledOnce();
  expect(useLaravelNavigation.getState().kind).toBeNull();
  resolve(ok());
  await pending;
  expect(mock.open).not.toHaveBeenCalled();
});
it("does not open a displayed handler after a caret change while selection waits for pending edits", async () => {
  mock.handler.mockResolvedValue(ok(true));
  await openLaravelNavigation("handler");
  const item = useLaravelNavigation.getState().items[0]!;
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const dispose = registerPendingEdits(() => barrier);
  try {
    const pending = selectLaravelItem(item);
    mock.caret.mockReturnValue({ ...mock.caret(), line: 5 });
    release();
    await pending;
    expect(mock.open).not.toHaveBeenCalled();
    expect(useLaravelNavigation.getState().kind).toBeNull();
  } finally {
    dispose();
  }
});
it("keeps route-name browsing usable when its source caret changes", async () => {
  mock.routes.mockResolvedValueOnce({
    status: "ok",
    data: { routes: [{ name: "home", uri: "/", parameters: [], fromVendor: false }], locations: [], incomplete: false },
  });
  await openLaravelNavigation("routes");
  mock.caret.mockReturnValue({ ...mock.caret(), col: 27 });
  _emitEditorEvent({ type: "caretMoved", doc: 7, path: "/p/routes/web.php", line: 4, col: 27, offset: 101 });
  expect(useLaravelNavigation.getState().kind).toBe("routes");
  expect(useLaravelNavigation.getState().items[0]!.name).toBe("home");
});

it("quietly retires a late handler error after the source caret moved", async () => {
  let reject!: (error: unknown) => void;
  mock.handler.mockImplementation(
    () =>
      new Promise((_done, fail) => {
        reject = fail;
      }),
  );
  const pending = openLaravelNavigation("handler");
  await waitFor(() => expect(reject).toBeTypeOf("function"));
  mock.caret.mockReturnValue({ ...mock.caret(), col: 27 });
  reject(new Error("old lookup failed"));
  await pending;
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(useLaravelNavigation.getState().error).toBeNull();
  expect(mock.open).not.toHaveBeenCalled();
});
