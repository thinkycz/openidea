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
  mock.cancel.mockResolvedValue(undefined);
  mock.open.mockResolvedValue(undefined);
  mock.caret.mockReturnValue({ doc: 7, path: "/p/main.php", line: 0, col: 0, word: null, view: null });
  useProject.setState({
    project: { root: "/p", name: "p", isGit: false, settings: {} } as ProjectInfo,
    opening: false,
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

it("cancels and discards a late response when the project changes", async () => {
  let resolve!: (value: unknown) => void;
  mock.views.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = openLaravelNavigation("views");
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
  expect(mock.views).toHaveBeenCalledTimes(2);
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
