import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ProjectInfo } from "@/bindings";

const mock = vi.hoisted(() => ({ routes: vi.fn(), views: vi.fn(), cancel: vi.fn(), open: vi.fn(), hint: vi.fn() }));
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
  caretContext: () => ({ doc: 7, path: "/p/main.php", line: 0, col: 0, word: null, view: null }),
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
