import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ProjectInfo } from "@/bindings";

const mock = vi.hoisted(() => ({
  routes: vi.fn(),
  views: vi.fn(),
  cancel: vi.fn(),
  open: vi.fn(),
  hint: vi.fn(),
  caret: vi.fn(),
  language: null as string | null,
}));
vi.mock("@/editor/api", async (orig) => ({
  ...(await orig<typeof import("@/editor/api")>()),
  activeEditor: () => ({
    ctx: {
      doc: mock.caret()?.doc,
      path: mock.caret()?.path,
      language: mock.language ?? (mock.caret()?.path.endsWith(".json") ? "json" : "php"),
    },
    view: mock.caret()?.view,
  }),
}));
vi.mock("@/platform/ipc", async (orig) => {
  const actual = await orig<typeof import("@/platform/ipc")>();
  return {
    ...actual,
    isTauri: () => true,
    commands: {
      ...actual.commands,
      lspLaravelRoutes: mock.routes,
      lspLaravelViews: mock.views,
      cancelRequest: mock.cancel,
    },
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
  selectLaravelItem,
  useLaravelNavigation,
} = await import("./laravel");
const { LaravelNavigation } = await import("./LaravelNavigation");
const ok = <T,>(data: T) => Promise.resolve({ status: "ok" as const, data });
const loc = (line: number) => ({
  path: "/p/routes/web.php",
  range: { start: { line, col: 0 }, end: { line, col: 5 } },
});
let uninstall: () => void;
beforeEach(() => {
  vi.clearAllMocks();
  mock.language = null;
  mock.cancel.mockResolvedValue(undefined);
  mock.open.mockResolvedValue(undefined);
  mock.caret.mockReturnValue({ doc: 7, path: "/p/main.php", line: 0, col: 0, word: null, view: null });
  useProject.setState({
    project: { root: "/p", name: "p", isGit: false, settings: {} } as ProjectInfo,
    opening: false,
    settings: null,
  });
  uninstall = installLaravelNavigationEvents();
  mock.routes.mockImplementation((_request, _doc, name) =>
    ok(
      name === null
        ? {
            routes: [{ name: "invoice.show", uri: "invoices/{invoice}", parameters: ["invoice"], fromVendor: false }],
            locations: [],
            incomplete: true,
          }
        : { routes: [], locations: [loc(2), loc(8)], incomplete: false },
    ),
  );
  mock.views.mockResolvedValue({ status: "ok", data: { views: ["invoices.show"], locations: [], incomplete: false } });
});
afterEach(() => {
  uninstall();
  act(() => closeLaravelNavigation());
});

it("keeps duplicate route targets and incomplete warnings through selection", async () => {
  await openLaravelNavigation("routes");
  expect(mock.routes).toHaveBeenCalledWith(expect.any(Number), 7, null, 2000);
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  expect(useLaravelNavigation.getState().items).toHaveLength(2);
  expect(useLaravelNavigation.getState().incomplete).toBe(true);
  expect(mock.open).not.toHaveBeenCalled();
  await selectLaravelItem(useLaravelNavigation.getState().items[1]!);
  expect(mock.open).toHaveBeenCalledWith("/p/routes/web.php", 8, 0);
  expect(useLaravelNavigation.getState().kind).toBeNull();
});

it.each(["routes", "views"] as const)("uses the configured PHP language for %s navigation", async (kind) => {
  for (const path of ["/p/routes/custom.inc", "/p/route-index"]) {
    mock.caret.mockReturnValue({ ...mock.caret(), path });
    await openLaravelNavigation(kind);
    expect(useLaravelNavigation.getState().kind).toBe(kind);
    expect(kind === "routes" ? mock.routes : mock.views).toHaveBeenLastCalledWith(expect.any(Number), 7, null, 2000);
    expect(mock.hint).not.toHaveBeenCalled();
    closeLaravelNavigation();
  }
});

it.each(["routes", "views"] as const)("rejects non-PHP and virtual PHP sources for %s navigation", async (kind) => {
  mock.language = "plaintext";
  await openLaravelNavigation(kind);
  mock.language = "php";
  mock.caret.mockReturnValue({ ...mock.caret(), path: "phpantom-builtin:///stubs/Core.php" });
  await openLaravelNavigation(kind);
  expect(mock.routes).not.toHaveBeenCalled();
  expect(mock.views).not.toHaveBeenCalled();
  expect(mock.hint).toHaveBeenCalledTimes(2);
  expect(useLaravelNavigation.getState().kind).toBeNull();
});

it("cancels and discards a late response when the project changes", async () => {
  let resolve!: (value: unknown) => void;
  mock.views.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("views");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  useProject.setState({ project: null });
  expect(mock.cancel).toHaveBeenCalledOnce();
  resolve({ status: "ok", data: { views: ["stale"], locations: [], incomplete: false } });
  await pending;
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(useLaravelNavigation.getState().items).toEqual([]);
});

it("ignores a canceled previous query when a newer chooser is open", async () => {
  let resolve!: (value: unknown) => void;
  mock.routes.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("routes");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  await openLaravelNavigation("views");
  resolve({ status: "ok", data: { routes: [], locations: [loc(99)], incomplete: true } });
  await pending;
  expect(useLaravelNavigation.getState().items[0]!.name).toBe("invoices.show");
  expect(useLaravelNavigation.getState().incomplete).toBe(false);
});

it("shows static/incomplete notices, filters results, and cancels through the dialog", async () => {
  await openLaravelNavigation("routes");
  render(<LaravelNavigation />);
  expect(screen.getByText(/Results are incomplete/)).toBeInTheDocument();
  expect(screen.getByText(/Static source index/)).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Filter Laravel results"), { target: { value: "missing" } });
  expect(screen.getByText("No static matches found")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(useLaravelNavigation.getState().kind).toBeNull();
});

it("opens the clicked duplicate target while another row is active", async () => {
  await openLaravelNavigation("routes");
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  render(<LaravelNavigation />);
  fireEvent.click(screen.getAllByRole("option")[1]!);
  await waitFor(() => expect(mock.open).toHaveBeenCalledWith("/p/routes/web.php", 8, 0));
});

it("retries a failed route list without closing the chooser", async () => {
  mock.routes.mockResolvedValueOnce({
    status: "error",
    error: { kind: "unavailable", message: "PHP server is restarting" },
  });
  await openLaravelNavigation("routes");
  render(<LaravelNavigation />);
  expect(screen.getByRole("alert")).toHaveTextContent("PHP server is restarting");
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.getByRole("option")).toHaveTextContent("invoice.show"));
  expect(mock.routes).toHaveBeenCalledTimes(2);
  expect(mock.routes.mock.calls[1]).toEqual([expect.any(Number), 7, null, 2000]);
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("retries the selected name after target lookup fails and preserves incomplete results", async () => {
  await openLaravelNavigation("routes");
  mock.routes.mockResolvedValueOnce({
    status: "error",
    error: { kind: "unavailable", message: "PHP server is restarting" },
  });
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  render(<LaravelNavigation />);
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(2));
  expect(mock.routes.mock.calls[2]).toEqual([expect.any(Number), 7, "invoice.show", 2000]);
  expect(screen.getByText(/Results are incomplete/)).toBeInTheDocument();
  fireEvent.click(screen.getAllByRole("option")[1]!);
  await waitFor(() => expect(mock.open).toHaveBeenCalledWith("/p/routes/web.php", 8, 0));
});

it("cancels a pending view retry on project replacement and ignores its late response", async () => {
  mock.views.mockResolvedValueOnce({
    status: "error",
    error: { kind: "unavailable", message: "PHP server is restarting" },
  });
  await openLaravelNavigation("views");
  let resolve!: (value: unknown) => void;
  mock.views.mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  render(<LaravelNavigation />);
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(mock.views).toHaveBeenCalledTimes(2));
  expect(screen.getByRole("button", { name: "Show Targets" })).toBeDisabled();
  act(() => useProject.setState({ project: null }));
  expect(mock.cancel).toHaveBeenCalledOnce();
  await act(async () => resolve({ status: "ok", data: { views: ["stale"], locations: [], incomplete: false } }));
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(mock.open).not.toHaveBeenCalled();
});

it("allows an explicit retry when indexing cancels the current lookup", async () => {
  mock.views.mockResolvedValueOnce({
    status: "error",
    error: { kind: "cancelled", message: "Request cancelled" },
  });
  await openLaravelNavigation("views");
  expect(useLaravelNavigation.getState().loading).toBe(false);
  render(<LaravelNavigation />);
  expect(screen.getByRole("alert")).toHaveTextContent("Cancelled");
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.getByRole("option")).toHaveTextContent("invoices.show"));
});

it("returns from route targets to a fresh name list without opening a file", async () => {
  await openLaravelNavigation("routes");
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  render(<LaravelNavigation />);
  mock.routes.mockResolvedValueOnce({
    status: "ok",
    data: {
      routes: [{ name: "invoice.index", uri: "invoices", parameters: [], fromVendor: false }],
      locations: [],
      incomplete: false,
    },
  });
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await waitFor(() => expect(screen.getByRole("option")).toHaveTextContent("invoice.index"));
  expect(mock.routes.mock.calls[2]).toEqual([expect.any(Number), 7, null, 2000]);
  expect(useLaravelNavigation.getState().selectedName).toBeNull();
  expect(screen.getByText(/Results are incomplete/)).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Back" })).not.toBeInTheDocument();
  expect(mock.open).not.toHaveBeenCalled();
});

it("Back cancels a pending view target lookup and ignores its late result", async () => {
  await openLaravelNavigation("views");
  let resolve!: (value: unknown) => void;
  mock.views.mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const targetLookup = selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  render(<LaravelNavigation />);
  mock.views.mockResolvedValueOnce({ status: "ok", data: { views: ["dashboard"], locations: [], incomplete: false } });
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await waitFor(() => expect(screen.getByRole("option")).toHaveTextContent("dashboard"));
  expect(mock.cancel).toHaveBeenCalledWith(mock.views.mock.calls[1]![0]);
  await act(async () => {
    resolve({ status: "ok", data: { views: [], locations: [loc(99)], incomplete: true } });
    await targetLookup;
  });
  expect(useLaravelNavigation.getState().phase).toBe("names");
  expect(useLaravelNavigation.getState().incomplete).toBe(false);
  expect(screen.getByRole("option")).toHaveTextContent("dashboard");
  expect(mock.open).not.toHaveBeenCalled();
});

it("returns to route names after selected-name lookup fails", async () => {
  await openLaravelNavigation("routes");
  mock.routes.mockResolvedValueOnce({ status: "error", error: { kind: "unavailable", message: "Server unavailable" } });
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  render(<LaravelNavigation />);
  expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await waitFor(() => expect(screen.getByRole("option")).toHaveTextContent("invoice.show"));
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Show Targets" })).toBeEnabled();
});

it("ignores a queued target activation after Back replaced its result list", async () => {
  await openLaravelNavigation("routes");
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  const oldTarget = useLaravelNavigation.getState().items[1]!;
  render(<LaravelNavigation />);
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await waitFor(() => expect(useLaravelNavigation.getState().phase).toBe("names"));
  await selectLaravelItem(oldTarget);
  expect(mock.open).not.toHaveBeenCalled();
  expect(screen.getByRole("dialog")).toBeInTheDocument();
  expect(useLaravelNavigation.getState().phase).toBe("names");
});

it("refuses a former project's target without dismissing the current views chooser", async () => {
  await openLaravelNavigation("routes");
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  const oldTarget = useLaravelNavigation.getState().items[0]!;
  act(() => useProject.setState({ project: { root: "/q", name: "q", isGit: false, settings: {} } as ProjectInfo }));
  mock.caret.mockReturnValue({ doc: 8, path: "/q/main.php", line: 0, col: 0, word: null, view: null });
  await openLaravelNavigation("views");
  const current = useLaravelNavigation.getState().items;
  await selectLaravelItem(oldTarget);
  expect(mock.open).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().kind).toBe("views");
  expect(useLaravelNavigation.getState().items).toBe(current);
  expect(mock.views).toHaveBeenCalledWith(expect.any(Number), 8, null, 2000);
});

it("ignores stale name activations and accepts only the new chooser's results", async () => {
  await openLaravelNavigation("routes");
  const oldName = useLaravelNavigation.getState().items[0]!;
  await openLaravelNavigation("views");
  const currentName = useLaravelNavigation.getState().items[0]!;
  const calls = mock.views.mock.calls.length;
  await selectLaravelItem(oldName);
  expect(mock.views).toHaveBeenCalledTimes(calls);
  expect(useLaravelNavigation.getState().phase).toBe("names");
  mock.views.mockResolvedValueOnce({
    status: "ok",
    data: {
      views: [],
      locations: [{ ...loc(4), path: "/p/resources/views/invoices/show.blade.php" }],
      incomplete: false,
    },
  });
  await selectLaravelItem(currentName);
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  expect(mock.open).toHaveBeenCalledWith("/p/resources/views/invoices/show.blade.php", 4, 0);
});

it.each(["routes", "views"] as const)(
  "retires displayed %s targets after an observed file change and retries the same name",
  async (kind) => {
    if (kind === "views")
      mock.views.mockImplementation((_request, _doc, name) =>
        ok(
          name === null
            ? { views: ["invoices.show"], locations: [], incomplete: true }
            : { views: [], locations: [loc(2)], incomplete: false },
        ),
      );
    await openLaravelNavigation(kind);
    const name = useLaravelNavigation.getState().items[0]!.name;
    await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
    const stale = useLaravelNavigation.getState().items[0]!;
    render(<LaravelNavigation />);
    const { emitLocalIdeEvent } = await import("@/platform/ipc");
    act(() => emitLocalIdeEvent({ type: "fsChanged", created: [], modified: ["/p/routes/web.php"], removed: [] }));
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByRole("alert")).toHaveTextContent("Project files changed");
    expect(useLaravelNavigation.getState().selectedName).toBe(name);
    expect(screen.getByText(/Results are incomplete/)).toBeInTheDocument();
    await selectLaravelItem(stale);
    expect(mock.open).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(screen.getAllByRole("option").length).toBeGreaterThan(0));
    expect((kind === "routes" ? mock.routes : mock.views).mock.calls.at(-1)).toEqual([
      expect.any(Number),
      7,
      name,
      2000,
    ]);
    await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
    expect(mock.open).toHaveBeenCalledWith("/p/routes/web.php", 2, 0);
  },
);

it("retires pending names after a file creation and ignores their late response", async () => {
  let resolve!: (value: unknown) => void;
  mock.views.mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("views");
  const { emitLocalIdeEvent } = await import("@/platform/ipc");
  emitLocalIdeEvent({ type: "fsChanged", created: ["/p/resources/views/new.blade.php"], modified: [], removed: [] });
  expect(mock.cancel).toHaveBeenCalledOnce();
  resolve({ status: "ok", data: { views: ["stale"], locations: [], incomplete: false } });
  await pending;
  expect(useLaravelNavigation.getState().items).toEqual([]);
  expect(useLaravelNavigation.getState().loading).toBe(false);
  expect(useLaravelNavigation.getState().error).toContain("Project files changed");
  render(<LaravelNavigation />);
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.getByRole("option")).toHaveTextContent("invoices.show"));
});

it("does not retire results for empty file batches or after disposing its event subscriptions", async () => {
  await openLaravelNavigation("routes");
  const items = useLaravelNavigation.getState().items;
  const { emitLocalIdeEvent } = await import("@/platform/ipc");
  emitLocalIdeEvent({ type: "fsChanged", created: [], modified: [], removed: [] });
  expect(useLaravelNavigation.getState().items).toBe(items);
  expect(useLaravelNavigation.getState().error).toBeNull();
  uninstall();
  emitLocalIdeEvent({ type: "fsChanged", created: [], modified: [], removed: ["/p/routes/web.php"] });
  expect(useLaravelNavigation.getState().items).toBe(items);
});

it.each(["routes", "views"] as const)(
  "retires displayed %s targets when PHPantom stops and explicitly retries the same name",
  async (kind) => {
    if (kind === "views")
      mock.views.mockImplementation((_request, _doc, name) =>
        ok(
          name === null
            ? { views: ["invoices.show"], locations: [], incomplete: true }
            : { views: [], locations: [loc(2)], incomplete: false },
        ),
      );
    await openLaravelNavigation(kind);
    await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
    const stale = useLaravelNavigation.getState().items[0]!;
    const name = stale.name;
    render(<LaravelNavigation />);
    const { emitLocalIdeEvent } = await import("@/platform/ipc");
    act(() =>
      emitLocalIdeEvent({ type: "languageServer", server: 31, name: "phpantom", status: "stopped", message: null }),
    );
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByRole("alert")).toHaveTextContent("PHP server");
    expect(screen.getByText(/Results are incomplete/)).toBeInTheDocument();
    expect(useLaravelNavigation.getState().selectedName).toBe(name);
    await selectLaravelItem(stale);
    expect(mock.open).not.toHaveBeenCalled();
    act(() =>
      emitLocalIdeEvent({ type: "languageServer", server: 31, name: "phpantom", status: "ready", message: null }),
    );
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(screen.getAllByRole("option").length).toBeGreaterThan(0));
    expect((kind === "routes" ? mock.routes : mock.views).mock.calls.at(-1)).toEqual([
      expect.any(Number),
      7,
      name,
      2000,
    ]);
    await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
    expect(mock.open).toHaveBeenCalledWith("/p/routes/web.php", 2, 0);
  },
);

it.each(["stopped", "crashed", "missing", "misconfigured"] as const)(
  "cancels a pending Laravel lookup after PHPantom becomes %s and ignores its late reply",
  async (status) => {
    let resolve!: (value: unknown) => void;
    mock.routes.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const pending = openLaravelNavigation("routes");
    const { emitLocalIdeEvent } = await import("@/platform/ipc");
    emitLocalIdeEvent({ type: "languageServer", server: 31, name: "phpantom", status, message: null });
    expect(mock.cancel).toHaveBeenCalledWith(mock.routes.mock.calls[0]![0]);
    resolve({
      status: "ok",
      data: {
        routes: [{ name: "stale", uri: "stale", parameters: [], fromVendor: false }],
        locations: [],
        incomplete: false,
      },
    });
    await pending;
    expect(useLaravelNavigation.getState().items).toEqual([]);
    expect(useLaravelNavigation.getState().loading).toBe(false);
    expect(useLaravelNavigation.getState().error).toContain("PHP server");
  },
);

it("allows initial PHPantom startup and ordinary progress, and ignores other servers or disposed listeners", async () => {
  const { emitLocalIdeEvent } = await import("@/platform/ipc");
  let resolve!: (value: unknown) => void;
  mock.views.mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("views");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  for (const status of ["starting", "busy", "ready"] as const)
    emitLocalIdeEvent({ type: "languageServer", server: 31, name: "phpantom", status, message: null });
  expect(mock.cancel).not.toHaveBeenCalled();
  expect(useLaravelNavigation.getState().loading).toBe(true);
  resolve({ status: "ok", data: { views: ["invoices.show"], locations: [], incomplete: false } });
  await pending;
  const items = useLaravelNavigation.getState().items;
  emitLocalIdeEvent({
    type: "languageServer",
    server: 42,
    name: "typescript-language-server",
    status: "crashed",
    message: null,
  });
  expect(useLaravelNavigation.getState().items).toBe(items);
  uninstall();
  emitLocalIdeEvent({ type: "languageServer", server: 31, name: "phpantom", status: "stopped", message: null });
  expect(useLaravelNavigation.getState().items).toBe(items);
});

it.each([
  ["routes", "/p/lang/cs.json"],
  ["views", "/p/lang/cs.json"],
  ["routes", "/p/resources/lang/en.json"],
  ["views", "/p/resources/lang/en.json"],
  ["routes", "/p/tools/child-🐘/lang/en.json"],
  ["views", "/p/tools/child-🐘/lang/en.json"],
  ["routes", "/p/tools/child-🐘/resources/lang/en.json"],
  ["views", "/p/tools/child-🐘/resources/lang/en.json"],
] as const)("opens %s targets from Laravel translation JSON %s", async (kind, path) => {
  mock.caret.mockReturnValue({ doc: 7, path, line: 0, col: 3, word: null, view: null });
  mock.views.mockImplementation((_request, _doc, name) =>
    ok(
      name === null
        ? { views: ["invoices.show"], locations: [], incomplete: false }
        : {
            views: [],
            locations: [{ ...loc(0), path: "/p/resources/views/invoices/show.blade.php" }],
            incomplete: false,
          },
    ),
  );
  await openLaravelNavigation(kind);
  expect(kind === "routes" ? mock.routes : mock.views).toHaveBeenCalledWith(expect.any(Number), 7, null, 2000);
  expect(mock.hint).not.toHaveBeenCalled();
  render(<LaravelNavigation />);
  fireEvent.click(screen.getByRole("button", { name: "Show Targets" }));
  await waitFor(() => expect(useLaravelNavigation.getState().phase).toBe("locations"));
  fireEvent.click(screen.getByRole("button", { name: "Open" }));
  await waitFor(() =>
    expect(mock.open).toHaveBeenCalledWith(
      kind === "routes" ? "/p/routes/web.php" : "/p/resources/views/invoices/show.blade.php",
      kind === "routes" ? 2 : 0,
      0,
    ),
  );
});

it.each(["/p/composer.json", "/p/config/options.json", "/p/lang/nested/en.json", "/other/lang/en.json"])(
  "does not treat unrelated JSON %s as a Laravel navigation source",
  async (path) => {
    mock.caret.mockReturnValue({ doc: 7, path, line: 0, col: 0, word: null, view: null });
    await openLaravelNavigation("routes");
    expect(mock.routes).not.toHaveBeenCalled();
    expect(mock.hint).toHaveBeenCalled();
    expect(useLaravelNavigation.getState().kind).toBeNull();
  },
);

it.each(["/p-other/lang/en.json", "/p/../other/lang/en.json"])(
  "keeps nested JSON navigation inside the project boundary %s",
  async (path) => {
    mock.caret.mockReturnValue({ doc: 7, path, line: 0, col: 0, word: null, view: null });
    await openLaravelNavigation("views");
    expect(mock.views).not.toHaveBeenCalled();
    expect(mock.hint).toHaveBeenCalled();
    expect(useLaravelNavigation.getState().kind).toBeNull();
  },
);

it.each(["routes", "views"] as const)("opens %s from an explicitly configured external content root", async (kind) => {
  useProject.setState({ settings: { projectStructure: { contentRoots: ["/linked"] } } as ProjectInfo["settings"] });
  mock.caret.mockReturnValue({
    doc: 7,
    path: "/linked/tools/app/lang/en.json",
    line: 0,
    col: 0,
    word: null,
    view: null,
  });
  await openLaravelNavigation(kind);
  expect(kind === "routes" ? mock.routes : mock.views).toHaveBeenCalledWith(expect.any(Number), 7, null, 2000);
  expect(mock.hint).not.toHaveBeenCalled();
});

it("retires displayed rows and ignores queued activation when content roots change", async () => {
  await openLaravelNavigation("routes");
  const item = useLaravelNavigation.getState().items[0]!;
  useProject.setState({ settings: { projectStructure: { contentRoots: ["/linked"] } } as ProjectInfo["settings"] });
  expect(useLaravelNavigation.getState().kind).toBeNull();
  await selectLaravelItem(item);
  expect(mock.routes).toHaveBeenCalledTimes(1);
  expect(mock.open).not.toHaveBeenCalled();
});

it("cancels a pending chooser and discards its reply after content-root removal", async () => {
  useProject.setState({ settings: { projectStructure: { contentRoots: ["/linked"] } } as ProjectInfo["settings"] });
  let resolve!: (value: unknown) => void;
  mock.routes.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("routes");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  useProject.setState({ settings: { projectStructure: { contentRoots: [] } } as ProjectInfo["settings"] });
  expect(mock.cancel).toHaveBeenCalledTimes(1);
  resolve({
    status: "ok",
    data: {
      routes: [{ name: "stale", uri: "stale", parameters: [], fromVendor: false }],
      locations: [],
      incomplete: false,
    },
  });
  await pending;
  expect(useLaravelNavigation.getState().kind).toBeNull();
  expect(useLaravelNavigation.getState().items).toEqual([]);
});

it("keeps rows for unrelated settings and equivalent content-root sets", async () => {
  useProject.setState({ settings: { projectStructure: { contentRoots: ["/b", "/a"] } } as ProjectInfo["settings"] });
  await openLaravelNavigation("routes");
  const items = useLaravelNavigation.getState().items;
  useProject.setState({
    settings: {
      projectStructure: { contentRoots: ["/a", "/b", "/a"] },
      files: { autoSave: false },
    } as ProjectInfo["settings"],
  });
  expect(useLaravelNavigation.getState().items).toBe(items);
  expect(useLaravelNavigation.getState().kind).toBe("routes");
});

it.each(["/linked-other/lang/en.json", "/linked/../outside/lang/en.json"])(
  "refuses external-root boundary candidate %s",
  async (path) => {
    useProject.setState({ settings: { projectStructure: { contentRoots: ["/linked"] } } as ProjectInfo["settings"] });
    mock.caret.mockReturnValue({ doc: 7, path, line: 0, col: 0, word: null, view: null });
    await openLaravelNavigation("routes");
    expect(mock.routes).not.toHaveBeenCalled();
    expect(mock.hint).toHaveBeenCalled();
  },
);

it("ignores project content-root settings in workspace mode", async () => {
  const project = useProject.getState().project!;
  useProject.setState({
    project: { ...project, workspace: { version: 1, name: "workspace", projects: ["/p"] } },
    settings: { projectStructure: { contentRoots: ["/linked"] } } as ProjectInfo["settings"],
  });
  mock.caret.mockReturnValue({ doc: 7, path: "/linked/lang/en.json", line: 0, col: 0, word: null, view: null });
  await openLaravelNavigation("routes");
  expect(mock.routes).not.toHaveBeenCalled();
});

it.each(["routes", "views"] as const)("trims configured root whitespace for %s like the host", async (kind) => {
  useProject.setState({ settings: { projectStructure: { contentRoots: [" /linked/ "] } } as ProjectInfo["settings"] });
  mock.caret.mockReturnValue({ doc: 7, path: "/linked/lang/en.json", line: 0, col: 0, word: null, view: null });
  await openLaravelNavigation(kind);
  expect(kind === "routes" ? mock.routes : mock.views).toHaveBeenCalledWith(expect.any(Number), 7, null, 2000);
});

it("preserves literal trailing spaces in the primary project path", async () => {
  const project = useProject.getState().project!;
  useProject.setState({ project: { ...project, root: "/primary space " } });
  mock.caret.mockReturnValue({ doc: 7, path: "/primary space /lang/en.json", line: 0, col: 0, word: null, view: null });
  await openLaravelNavigation("routes");
  expect(mock.routes).toHaveBeenCalledWith(expect.any(Number), 7, null, 2000);
});

it.each(["routes", "views"] as const)(
  "retires displayed %s targets after a real unsaved sibling edit and retries the same name",
  async (kind) => {
    const { createMemoryBackend } = await import("@/editor/core/backend");
    const { DocumentManager } = await import("@/editor/core/model/document");
    const { EditorSlot } = await import("@/editor/core/model/slot");
    const backend = createMemoryBackend({
      "/p/routes/web.php": "<?php\nRoute::get('/invoices', 'InvoiceController');",
    });
    const scheduled: Array<() => void> = [];
    const manager = new DocumentManager(backend, { settings: () => ({}), notify: vi.fn() }, (fn) => scheduled.push(fn));
    manager.start();
    const model = await manager.acquire("/p/routes/web.php");
    const slot = new EditorSlot(model, "routes", {
      onFocus: () => {},
      onBlur: () => {},
      onSelection: () => {},
      onUserEdit: () => {},
      onContextMenu: () => {},
    });
    if (kind === "views")
      mock.views.mockImplementation((_request, _doc, name) =>
        ok(
          name === null
            ? { views: ["invoices.show"], locations: [], incomplete: true }
            : { views: [], locations: [loc(2)], incomplete: false },
        ),
      );
    try {
      await openLaravelNavigation(kind);
      await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
      const stale = useLaravelNavigation.getState().items[0]!;
      const name = useLaravelNavigation.getState().selectedName;
      render(<LaravelNavigation />);
      act(() => slot.dispatchLocal({ changes: { from: 0, insert: "// unsaved\n" }, userEvent: "input.type" }));
      await act(async () => {
        scheduled.splice(0).forEach((fn) => fn());
        await model.sync.whenIdle();
      });
      expect((await backend.snapshot(model.doc)).text).toContain("// unsaved");
      expect(screen.queryAllByRole("option")).toHaveLength(0);
      expect(screen.getByRole("alert")).toHaveTextContent("Project files changed");
      expect(useLaravelNavigation.getState().selectedName).toBe(name);
      await selectLaravelItem(stale);
      expect(mock.open).not.toHaveBeenCalled();
      fireEvent.click(screen.getByRole("button", { name: "Retry" }));
      await waitFor(() => expect(screen.getAllByRole("option").length).toBeGreaterThan(0));
      expect((kind === "routes" ? mock.routes : mock.views).mock.calls.at(-1)).toEqual([
        expect.any(Number),
        7,
        name,
        2000,
      ]);
      expect(screen.getByText(/Results are incomplete/)).toBeInTheDocument();
    } finally {
      manager.stop();
    }
  },
);

it.each(["docPatched", "docMoved"] as const)(
  "retires pending queries on %s and ignores the late reply",
  async (type) => {
    let resolve!: (value: unknown) => void;
    mock.views.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const pending = openLaravelNavigation("views");
    const { emitLocalIdeEvent } = await import("@/platform/ipc");
    emitLocalIdeEvent(
      type === "docPatched"
        ? { type, doc: 8, baseVersion: 1, version: 2, changes: [{ from: 0, to: 0, insert: "\n" }] }
        : { type, doc: 8, from: "/p/routes/web.php", to: "/p/routes/api.php" },
    );
    expect(mock.cancel).toHaveBeenCalledOnce();
    resolve({ status: "ok", data: { views: ["stale"], locations: [], incomplete: false } });
    await pending;
    expect(useLaravelNavigation.getState().items).toEqual([]);
    expect(useLaravelNavigation.getState().error).toContain("Project files changed");
  },
);

it.each(["docClosed", "docMoved"] as const)("closes the chooser when its source is %s", async (type) => {
  await openLaravelNavigation("routes");
  if (type === "docClosed") {
    const { _emitEditorEvent } = await import("@/editor/api");
    _emitEditorEvent({ type, doc: 7, path: "/p/main.php" });
  } else {
    const { emitLocalIdeEvent } = await import("@/platform/ipc");
    emitLocalIdeEvent({ type, doc: 7, from: "/p/main.php", to: "/p/renamed.php" });
  }
  expect(useLaravelNavigation.getState().kind).toBeNull();
});

it.each(["routes", "views"] as const)("discards pending %s names after an unsaved change", async (kind) => {
  const request = kind === "routes" ? mock.routes : mock.views;
  let resolve!: (value: unknown) => void;
  request.mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation(kind);
  const { _emitEditorEvent } = await import("@/editor/api");
  _emitEditorEvent({
    type: "docChanged",
    doc: 8,
    path: "/p/routes/web.php",
    baseVersion: 2,
    version: 3,
    changes: [{ from: 0, to: 0, insert: "\n" }],
  });
  expect(mock.cancel).toHaveBeenCalledOnce();
  resolve({ status: "ok", data: { routes: [], views: ["stale"], locations: [], incomplete: false } });
  await pending;
  expect(useLaravelNavigation.getState()).toMatchObject({ items: [], loading: false, selectedName: null });
  expect(useLaravelNavigation.getState().error).toContain("Project files changed");
});

it("retires sibling-overlay results on close but ignores caret events and disposed subscriptions", async () => {
  const { _emitEditorEvent } = await import("@/editor/api");
  await openLaravelNavigation("routes");
  const original = useLaravelNavigation.getState().items;
  _emitEditorEvent({ type: "caretMoved", doc: 7, path: "/p/main.php", line: 0, col: 0, offset: 0 });
  expect(useLaravelNavigation.getState().items).toBe(original);
  _emitEditorEvent({ type: "docClosed", doc: 8, path: "/p/routes/web.php" });
  expect(useLaravelNavigation.getState().items).toEqual([]);
  expect(useLaravelNavigation.getState().kind).toBe("routes");
  await openLaravelNavigation("views");
  const retained = useLaravelNavigation.getState().items;
  uninstall();
  _emitEditorEvent({
    type: "docChanged",
    doc: 7,
    path: "/p/main.php",
    baseVersion: 0,
    version: 1,
    changes: [{ from: 0, to: 0, insert: "\n" }],
  });
  const { emitLocalIdeEvent } = await import("@/platform/ipc");
  emitLocalIdeEvent({ type: "docPatched", doc: 7, baseVersion: 1, version: 2, changes: [] });
  emitLocalIdeEvent({ type: "docMoved", doc: 7, from: "/p/main.php", to: "/p/new.php" });
  _emitEditorEvent({ type: "docClosed", doc: 7, path: "/p/main.php" });
  expect(useLaravelNavigation.getState().items).toBe(retained);
  expect(useLaravelNavigation.getState().kind).toBe("views");
});

it.each(["routes", "views"] as const)("waits for all pending edits before querying %s", async (kind) => {
  const { registerPendingEdits, _emitEditorEvent } = await import("@/editor/api");
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const sync = vi.fn(() => barrier);
  const off = registerPendingEdits(sync);
  const request = kind === "routes" ? mock.routes : mock.views;
  const pending = openLaravelNavigation(kind);
  try {
    expect(sync).toHaveBeenCalledOnce();
    expect(request).not.toHaveBeenCalled();
    expect(useLaravelNavigation.getState()).toMatchObject({ loading: true, items: [], error: null });
    // An edit accepted during the barrier is part of the pending source snapshot.
    _emitEditorEvent({
      type: "docChanged",
      doc: 8,
      path: "/p/routes/web.php",
      baseVersion: 0,
      version: 1,
      changes: [{ from: 0, to: 0, insert: "\n" }],
    });
    release();
    await pending;
    expect(request).toHaveBeenCalledOnce();
    expect(useLaravelNavigation.getState().items.length).toBeGreaterThan(0);
    expect(useLaravelNavigation.getState().error).toBeNull();
  } finally {
    release();
    off();
    await pending;
  }
});

it.each(["close", "project", "replacement"] as const)(
  "never sends a query retired by %s while edits sync",
  async (reason) => {
    const { registerPendingEdits } = await import("@/editor/api");
    let release!: () => void;
    const barrier = new Promise<void>((resolve) => {
      release = resolve;
    });
    const off = registerPendingEdits(() => barrier);
    const pending = openLaravelNavigation("routes");
    let replacement: Promise<void> | undefined;
    try {
      if (reason === "close") closeLaravelNavigation();
      else if (reason === "project") useProject.setState({ project: null });
      else replacement = openLaravelNavigation("views");
      release();
      await pending;
      await replacement;
      expect(mock.routes).not.toHaveBeenCalled();
      expect(mock.cancel).not.toHaveBeenCalled();
      if (reason === "replacement") {
        expect(mock.views).toHaveBeenCalledOnce();
        expect(useLaravelNavigation.getState().items[0]?.name).toBe("invoices.show");
      } else expect(useLaravelNavigation.getState().kind).toBeNull();
    } finally {
      release();
      off();
      await pending;
      await replacement;
    }
  },
);

it("reports a failed sync without querying and retries the selected name after recovery", async () => {
  await openLaravelNavigation("routes");
  await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  const name = useLaravelNavigation.getState().selectedName;
  const { registerPendingEdits } = await import("@/editor/api");
  const off = registerPendingEdits(() => Promise.reject(new Error("Cannot synchronize pending edits")));
  const calls = mock.routes.mock.calls.length;
  const { emitLocalIdeEvent } = await import("@/platform/ipc");
  emitLocalIdeEvent({ type: "fsChanged", created: [], modified: ["/p/routes/web.php"], removed: [] });
  const { retryLaravelNavigation } = await import("./laravel");
  try {
    await retryLaravelNavigation();
    expect(mock.routes).toHaveBeenCalledTimes(calls);
    expect(useLaravelNavigation.getState()).toMatchObject({ selectedName: name, loading: false, incomplete: true });
    expect(useLaravelNavigation.getState().error).toContain("Cannot synchronize pending edits");
  } finally {
    off();
  }
  await retryLaravelNavigation();
  expect(mock.routes.mock.calls.at(-1)).toEqual([expect.any(Number), 7, name, 2000]);
  expect(useLaravelNavigation.getState().error).toBeNull();
});

it.each(["routes", "views"] as const)(
  "queries %s only after both real sibling edits are acknowledged",
  async (kind) => {
    const { createMemoryBackend } = await import("@/editor/core/backend");
    const { DocumentManager } = await import("@/editor/core/model/document");
    const { EditorSlot } = await import("@/editor/core/model/slot");
    const original = "<?php\nRoute::view('/invoices', 'invoices.show')->name('invoice.show');";
    const backend = createMemoryBackend({ "/p/routes/web.php": original });
    const edit = backend.edit.bind(backend);
    const acks: Array<() => void> = [];
    vi.spyOn(backend, "edit").mockImplementation(
      (...args) =>
        new Promise((resolve, reject) => {
          acks.push(() => {
            edit(...args).then(resolve, reject);
          });
        }),
    );
    const manager = new DocumentManager(backend, { settings: () => ({ files: { autoSave: false } }), notify: vi.fn() });
    manager.start();
    const model = await manager.acquire("/p/routes/web.php");
    const slot = new EditorSlot(model, "unsaved-routes", {
      onFocus: () => {},
      onBlur: () => {},
      onSelection: () => {},
      onUserEdit: () => {},
      onContextMenu: () => {},
    });
    slot.mount(document.body);
    const request = kind === "routes" ? mock.routes : mock.views;
    let queriedText: string | undefined;
    request.mockImplementationOnce(async () => {
      queriedText = (await backend.snapshot(model.doc)).text;
      return {
        status: "ok",
        data:
          kind === "routes"
            ? {
                routes: [{ name: "invoice.show", uri: "invoices", parameters: [], fromVendor: false }],
                locations: [],
                incomplete: false,
              }
            : { views: ["invoices.show"], locations: [], incomplete: false },
      };
    });
    let pending: Promise<void> | undefined;
    try {
      slot.dispatchLocal({ changes: { from: 6, insert: "// première\n" }, userEvent: "input.type" });
      model.sync.flush();
      expect(acks).toHaveLength(1);
      slot.dispatchLocal({ changes: { from: 6, insert: "// second\n" }, userEvent: "input.type" });
      pending = openLaravelNavigation(kind);
      expect(request).not.toHaveBeenCalled();
      acks.shift()!();
      await waitFor(() => expect(acks).toHaveLength(1));
      expect(request).not.toHaveBeenCalled();
      expect(useLaravelNavigation.getState().error).toBeNull();
      acks.shift()!();
      await pending;
      expect(request).toHaveBeenCalledOnce();
      expect(queriedText).toBe("<?php\n// second\n// première\n" + original.slice(6));
      expect(useLaravelNavigation.getState()).toMatchObject({ loading: false, error: null });
      expect(useLaravelNavigation.getState().items).toHaveLength(1);
    } finally {
      for (const ack of acks.splice(0)) ack();
      await pending;
      slot.dispose();
      manager.stop();
    }
  },
);

it("Back replaces a target query still syncing without sending the retired name", async () => {
  await openLaravelNavigation("views");
  const { registerPendingEdits } = await import("@/editor/api");
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const off = registerPendingEdits(() => barrier);
  const calls = mock.views.mock.calls.length;
  const target = selectLaravelItem(useLaravelNavigation.getState().items[0]!);
  const { backLaravelNavigation } = await import("./laravel");
  const back = backLaravelNavigation();
  try {
    expect(mock.views).toHaveBeenCalledTimes(calls);
    release();
    await Promise.all([target, back]);
    expect(mock.views).toHaveBeenCalledTimes(calls + 1);
    expect(mock.views.mock.calls.at(-1)).toEqual([expect.any(Number), 7, null, 2000]);
    expect(mock.cancel).not.toHaveBeenCalled();
    expect(useLaravelNavigation.getState()).toMatchObject({ phase: "names", selectedName: null, error: null });
  } finally {
    release();
    off();
    await Promise.all([target, back]);
  }
});

it("retains a retryable file-change warning when a waiting query is invalidated", async () => {
  const { registerPendingEdits } = await import("@/editor/api");
  let release!: () => void;
  const barrier = new Promise<void>((resolve) => {
    release = resolve;
  });
  const off = registerPendingEdits(() => barrier);
  const pending = openLaravelNavigation("routes");
  const { emitLocalIdeEvent } = await import("@/platform/ipc");
  try {
    emitLocalIdeEvent({ type: "fsChanged", created: [], modified: ["/p/routes/web.php"], removed: [] });
    release();
    await pending;
    expect(mock.routes).not.toHaveBeenCalled();
    expect(mock.cancel).not.toHaveBeenCalled();
    expect(useLaravelNavigation.getState()).toMatchObject({ loading: false, items: [] });
    expect(useLaravelNavigation.getState().error).toContain("Project files changed");
  } finally {
    release();
    off();
    await pending;
  }
  const { retryLaravelNavigation } = await import("./laravel");
  await retryLaravelNavigation();
  expect(mock.routes).toHaveBeenCalledOnce();
  expect(useLaravelNavigation.getState().error).toBeNull();
});

it.each(["routes", "views"] as const)(
  "flushes a real pending sibling edit before opening a displayed %s target",
  async (kind) => {
    const { createMemoryBackend } = await import("@/editor/core/backend");
    const { DocumentManager } = await import("@/editor/core/model/document");
    const { EditorSlot } = await import("@/editor/core/model/slot");
    const original = "<?php\nRoute::view('/invoices', 'invoices.show')->name('invoice.show');";
    const backend = createMemoryBackend({ "/p/routes/web.php": original });
    const edit = backend.edit.bind(backend);
    let acknowledge: (() => void) | undefined;
    vi.spyOn(backend, "edit").mockImplementation(
      (...args) =>
        new Promise((resolve, reject) => {
          acknowledge = () => {
            edit(...args).then(resolve, reject);
          };
        }),
    );
    const manager = new DocumentManager(backend, { settings: () => ({ files: { autoSave: false } }), notify: vi.fn() });
    manager.start();
    const model = await manager.acquire("/p/routes/web.php");
    const slot = new EditorSlot(model, "pending-target", {
      onFocus: () => {},
      onBlur: () => {},
      onSelection: () => {},
      onUserEdit: () => {},
      onContextMenu: () => {},
    });
    slot.mount(document.body);
    if (kind === "views")
      mock.views.mockImplementation((_request, _doc, name) =>
        ok(
          name === null
            ? { views: ["invoices.show"], locations: [], incomplete: false }
            : { views: [], locations: [loc(2)], incomplete: false },
        ),
      );
    let selection: Promise<void> | undefined;
    try {
      await openLaravelNavigation(kind);
      await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
      const item = useLaravelNavigation.getState().items[0]!;
      slot.dispatchLocal({ changes: { from: 6, insert: "// unsaved 😀\n" }, userEvent: "input.type" });
      selection = selectLaravelItem(item);
      expect(mock.open).not.toHaveBeenCalled();
      expect(useLaravelNavigation.getState().loading).toBe(true);
      await waitFor(() => expect(acknowledge).toBeTypeOf("function"));
      acknowledge!();
      await selection;
      expect((await backend.snapshot(model.doc)).text).toBe(
        original.slice(0, 6) + "// unsaved 😀\n" + original.slice(6),
      );
      expect(mock.open).not.toHaveBeenCalled();
      expect(useLaravelNavigation.getState()).toMatchObject({ loading: false, items: [], selectedName: item.name });
      expect(useLaravelNavigation.getState().error).toContain("Project files changed");
    } finally {
      model.sync.flush();
      acknowledge?.();
      await model.sync.whenIdle();
      await selection;
      slot.dispose();
      manager.stop();
    }
  },
);

it.each(["routes", "views"] as const)(
  "guards displayed %s target activation across pending edits, cancellation and sync failure",
  async (kind) => {
    const { registerPendingEdits } = await import("@/editor/api");
    const { backLaravelNavigation, retryLaravelNavigation } = await import("./laravel");
    if (kind === "views")
      mock.views.mockImplementation((_request, _doc, name) =>
        ok(
          name === null
            ? { views: ["invoices.show"], locations: [], incomplete: true }
            : { views: [], locations: [loc(2)], incomplete: false },
        ),
      );
    for (const reason of ["unchanged", "close", "back", "project", "replacement", "failure"] as const) {
      await openLaravelNavigation(kind);
      await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
      const item = useLaravelNavigation.getState().items[0]!;
      const opened = mock.open.mock.calls.length;
      let release!: () => void;
      let reject!: (error: Error) => void;
      const barrier = new Promise<void>((resolve, fail) => {
        release = resolve;
        reject = fail;
      });
      const off = registerPendingEdits(() => barrier);
      const selection = selectLaravelItem(item);
      let replacement: Promise<void> | undefined;
      try {
        expect(mock.open).toHaveBeenCalledTimes(opened);
        expect(useLaravelNavigation.getState().loading).toBe(true);
        if (reason === "close") closeLaravelNavigation();
        else if (reason === "back") replacement = backLaravelNavigation();
        else if (reason === "project") useProject.setState({ project: { ...useProject.getState().project! } });
        else if (reason === "replacement") replacement = openLaravelNavigation(kind === "routes" ? "views" : "routes");
        if (reason === "failure") reject(new Error("Pending edit failed"));
        else release();
        await selection;
        await replacement;
        expect(mock.open).toHaveBeenCalledTimes(opened + (reason === "unchanged" ? 1 : 0));
        if (reason === "failure") {
          expect(useLaravelNavigation.getState()).toMatchObject({ loading: false, items: [], selectedName: item.name });
          expect(useLaravelNavigation.getState().error).toBe("Pending edit failed");
        }
        if (reason === "back")
          expect(useLaravelNavigation.getState()).toMatchObject({ phase: "names", loading: false });
        if (reason === "replacement")
          expect(useLaravelNavigation.getState().kind).toBe(kind === "routes" ? "views" : "routes");
      } finally {
        release();
        off();
        await selection;
        await replacement;
      }
      if (reason === "failure") {
        await retryLaravelNavigation();
        expect(useLaravelNavigation.getState()).toMatchObject({ selectedName: item.name, loading: false, error: null });
        await selectLaravelItem(useLaravelNavigation.getState().items[0]!);
        expect(mock.open).toHaveBeenCalledTimes(opened + 1);
      }
    }
  },
);
