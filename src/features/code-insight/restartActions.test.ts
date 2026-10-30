import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ProjectInfo, ServerId, ServerInfo } from "@/bindings";
import { allActions, executeAction, getAction, isActionEnabled } from "@/platform/actions";
import { setContext } from "@/platform/context";
import { useProject } from "@/platform/project";
import { provideNotificationService, type NotificationService } from "@/workbench/contributions";

const mock = vi.hoisted(() => ({ list: vi.fn(), restart: vi.fn() }));
vi.mock("@/platform/ipc", async (original) => {
  const actual = await original<typeof import("@/platform/ipc")>();
  return {
    ...actual,
    isTauri: () => false,
    commands: { ...actual.commands, lspServers: mock.list, lspRestart: mock.restart },
  };
});
const { activate } = await import("./index");
const { useServers } = await import("./servers");
const ID = "code.restartLanguageServers";
const project = (root = "/p") => ({ root, name: "test", settings: {}, isGit: false }) as ProjectInfo;
const server = (id: number) =>
  ({ id: id as ServerId, name: "phpantom", languages: ["php"], status: "ready", root: "/p", pid: null }) as ServerInfo;
let dispose: () => void;
let notes: Parameters<NotificationService["notify"]>[0][];

beforeEach(() => {
  vi.resetAllMocks();
  mock.list.mockResolvedValue({ status: "ok", data: [server(7), server(8)] });
  mock.restart.mockResolvedValue({ status: "ok", data: null });
  notes = [];
  provideNotificationService({ notify: (note) => void notes.push(note) });
  useProject.setState({ project: project(), opening: false });
  setContext("projectOpen", true);
  dispose = activate();
});
afterEach(() => {
  dispose();
  expect(getAction(ID)).toBeUndefined();
  useProject.setState({ project: null });
  useServers.setState({ servers: new Map() });
  setContext("projectOpen", false);
});

it("exposes restart through the shared action registry and requires an open project", async () => {
  const action = allActions().find((a) => a.id === ID);
  expect(action).toMatchObject({ title: "Restart Language Servers", category: "Code" });
  if (!action) return;
  expect(action.hidden).not.toBe(true);
  expect(isActionEnabled(action)).toBe(true);
  setContext("projectOpen", false);
  expect(await executeAction(ID)).toBe(false);
  expect(mock.list).not.toHaveBeenCalled();
});

it("restarts the current backend list rather than a stale status snapshot", async () => {
  useServers.setState({ servers: new Map([[99 as ServerId, { ...server(99), message: null }]]) });
  expect(await executeAction(ID)).toBe(true);
  expect(mock.list).toHaveBeenCalledOnce();
  expect(mock.restart.mock.calls.map(([id]) => id)).toEqual([7, 8]);
  expect(notes).toEqual([]);
});

it("reports one restart failure while still awaiting the other servers", async () => {
  let resolve!: (value: unknown) => void;
  mock.restart.mockImplementation((id: number) =>
    id === 7
      ? Promise.resolve({ status: "error", error: { kind: "unavailable", message: "test server could not restart" } })
      : new Promise((done) => {
          resolve = done;
        }),
  );
  let finished = false;
  const pending = executeAction(ID).then((ran) => {
    finished = true;
    return ran;
  });
  await vi.waitFor(() => expect(mock.restart).toHaveBeenCalledTimes(2));
  expect(finished).toBe(false);
  resolve({ status: "ok", data: null });
  expect(await pending).toBe(true);
  expect(notes).toEqual([expect.objectContaining({ level: "error", title: "Failed to restart language server" })]);
  expect(String(notes[0]?.body)).toContain("test server could not restart");
});

it.each(["closed", "opening"])("does not use a stale action context when the project is %s", async (state) => {
  useProject.setState(state === "closed" ? { project: null } : { opening: true });
  expect(await executeAction(ID)).toBe(true);
  expect(mock.list).not.toHaveBeenCalled();
  expect(mock.restart).not.toHaveBeenCalled();
});

it.each(["/other", "/p"])("discards a server list after the project is replaced at %s", async (root) => {
  let resolve!: (value: unknown) => void;
  mock.list.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = executeAction(ID);
  expect(mock.list).toHaveBeenCalledOnce();
  useProject.setState({ project: project(root) });
  resolve({ status: "ok", data: [server(7)] });
  await pending;
  expect(mock.restart).not.toHaveBeenCalled();
  expect(notes).toEqual([]);
});

it("reports a failed server lookup without attempting a restart", async () => {
  mock.list.mockResolvedValue({ status: "error", error: { kind: "unavailable", message: "test lookup failed" } });
  expect(await executeAction(ID)).toBe(true);
  expect(mock.restart).not.toHaveBeenCalled();
  expect(notes).toEqual([expect.objectContaining({ level: "error", title: "Failed to restart language server" })]);
});
