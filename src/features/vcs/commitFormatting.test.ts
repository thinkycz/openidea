import { afterEach, expect, it, vi } from "vitest";
import type { DocumentInfo, ProjectInfo, Settings, WorkspaceEdit } from "@/bindings";
import * as ipc from "@/platform/ipc";
import { useProject } from "@/platform/project";
import { maybeEditorService } from "@/workbench/contributions";
import { runCommitChecks } from "./commitChecks";
import { useVcs } from "./store";

vi.mock("@/workbench/contributions", async (original) => ({
  ...(await original<object>()),
  maybeEditorService: vi.fn(),
}));

const project = useProject.getState();
const vcs = useVcs.getState();
afterEach(() => {
  vi.restoreAllMocks();
  useProject.setState(project, true);
  useVcs.setState(vcs, true);
});

type FormatResult = Awaited<ReturnType<typeof ipc.commands.lspFormatEdit>>;
const ok = <T>(data: T) => ({ status: "ok" as const, data });
const info = {
  id: 1,
  path: "/repo/a.php",
  version: 7,
  text: "<?php /* 😀 */  echo 1;\n",
  readOnly: false,
} as DocumentInfo;
const files = [{ path: info.path, exists: true }];

function setup() {
  useProject.setState({
    opening: false,
    openingPath: null,
    lightEdit: false,
    settings: {
      vcs: {
        commit: { reformat: true, optimizeImports: false, cleanup: false, analyzeCode: false, checkTodo: false },
        git: { warnCrlf: false, warnDetachedHead: false, warnLargeFiles: false, warnBadFileNames: false },
      },
    } as Settings,
  });
  useVcs.setState({ root: "/repo", repo: "/repo", status: [], branch: null });
  vi.spyOn(ipc, "isTauri").mockReturnValue(true);
  vi.spyOn(ipc.commands, "docOpen").mockResolvedValue(ok(info));
  const snapshot = vi.spyOn(ipc.commands, "docSnapshot").mockResolvedValue(ok(info));
  const save = vi.spyOn(ipc.commands, "docSave").mockResolvedValue(ok(null));
  const close = vi.spyOn(ipc.commands, "docClose").mockResolvedValue(ok(null));
  const apply = vi.spyOn(ipc.commands, "applyWorkspaceEdit").mockResolvedValue(ok(null));
  const saveAll = vi.fn().mockResolvedValue(undefined);
  vi.mocked(maybeEditorService).mockReturnValue({ saveAll } as unknown as NonNullable<
    ReturnType<typeof maybeEditorService>
  >);
  let release!: (value: FormatResult) => void;
  const request = vi.spyOn(ipc.commands, "lspFormatEdit").mockImplementation(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
  );
  const plan: WorkspaceEdit = {
    files: [
      {
        kind: "edit",
        path: info.path,
        version: info.version,
        edits: [{ range: { start: { line: 0, col: 15 }, end: { line: 0, col: 15 } }, new_text: "  " }],
      },
    ],
    guard: {
      projectRevision: "project",
      documentsRevision: "documents",
      diskRevisions: [[info.path, "disk"]],
      engineGenerations: [[1, 2]],
    },
    preconditions: [{ path: info.path, hash: "input" }],
  };
  return {
    snapshot,
    save,
    close,
    apply,
    saveAll,
    request,
    release: (value: FormatResult = ok(plan)) => release(value),
    plan,
  };
}

it("keeps the original input version and every host witness through before-commit formatting", async () => {
  const f = setup();
  const pending = runCommitChecks(files, false);
  await vi.waitFor(() => expect(f.request).toHaveBeenCalledOnce());
  expect(f.request).toHaveBeenCalledWith(info.id, info.version, info.path, { tabSize: 4, insertSpaces: true });
  f.release();
  await pending;
  expect(f.apply).toHaveBeenCalledWith(f.plan);
  expect(f.save).toHaveBeenCalledWith(info.id);
  expect(f.close).toHaveBeenCalledWith(info.id);
  expect(f.saveAll).toHaveBeenCalledOnce();
});

it.each(["project", "settings", "opening", "openingPath", "lightEdit"] as const)(
  "retires the pending operation after %s changes",
  async (change) => {
    const f = setup();
    const pending = runCommitChecks(files, false);
    await vi.waitFor(() => expect(f.request).toHaveBeenCalledOnce());
    if (change === "project") useProject.setState({ project: { root: "/other" } as ProjectInfo });
    if (change === "settings") useProject.setState({ settings: { ...useProject.getState().settings! } });
    if (change === "opening") useProject.setState({ opening: true });
    if (change === "openingPath") useProject.setState({ openingPath: "/other" });
    if (change === "lightEdit") useProject.setState({ lightEdit: true });
    f.release();
    await pending;
    expect(f.apply).not.toHaveBeenCalled();
    expect(f.save).not.toHaveBeenCalled();
    expect(f.close).not.toHaveBeenCalled();
    expect(f.saveAll).not.toHaveBeenCalled();
  },
);

it.each(["plan", "apply"])("preserves the buffer when the host refuses a stale %s", async (stage) => {
  const f = setup();
  if (stage === "apply")
    f.apply.mockResolvedValue({ status: "error", error: { kind: "outOfSync", message: "Changed input" } });
  const pending = runCommitChecks(files, false);
  await vi.waitFor(() => expect(f.request).toHaveBeenCalledOnce());
  if (stage === "plan") f.release({ status: "error", error: { kind: "outOfSync", message: "Changed input" } });
  else f.release();
  await pending;
  expect(f.apply).toHaveBeenCalledTimes(stage === "apply" ? 1 : 0);
  expect(f.save).not.toHaveBeenCalled();
  expect(f.close).toHaveBeenCalledWith(info.id);
});

it("filters protected regions without dropping revision witnesses", async () => {
  const f = setup();
  const text = "<?php\n// @formatter:off\necho 1;\n// @formatter:on\necho 2;\n";
  useProject.setState({
    settings: { ...useProject.getState().settings!, editor: { codeStyle: { formatterTagsEnabled: true } } } as Settings,
  });
  f.snapshot.mockResolvedValue(ok({ ...info, text }));
  f.plan.files = [
    {
      kind: "edit",
      path: info.path,
      version: info.version,
      edits: [
        { range: { start: { line: 2, col: 0 }, end: { line: 2, col: 0 } }, new_text: "  " },
        { range: { start: { line: 4, col: 0 }, end: { line: 4, col: 0 } }, new_text: "  " },
      ],
    },
  ];
  const pending = runCommitChecks(files, false);
  await vi.waitFor(() => expect(f.request).toHaveBeenCalledOnce());
  f.release();
  await pending;
  expect(f.apply).toHaveBeenCalledOnce();
  const applied = f.apply.mock.calls[0]![0];
  expect(applied.guard).toEqual(f.plan.guard);
  expect(applied.preconditions).toEqual(f.plan.preconditions);
  expect(applied.files[0]).toMatchObject({
    version: info.version,
    edits: [{ range: { start: { line: 4, col: 0 } }, new_text: "  " }],
  });
});

it.each(["open", "snapshot"])("does not dispatch formatting after a project switch during %s", async (stage) => {
  const f = setup();
  let release!: (value: ReturnType<typeof ok<DocumentInfo>>) => void;
  const held = stage === "open" ? vi.spyOn(ipc.commands, "docOpen") : f.snapshot;
  held.mockImplementation(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
  );
  const pending = runCommitChecks(files, false);
  await vi.waitFor(() => expect(held).toHaveBeenCalledOnce());
  useProject.setState({ project: { root: "/other" } as ProjectInfo });
  release(ok(info));
  await pending;
  expect(f.request).not.toHaveBeenCalled();
  expect(f.apply).not.toHaveBeenCalled();
  expect(f.save).not.toHaveBeenCalled();
  expect(f.close).not.toHaveBeenCalled();
  expect(f.saveAll).not.toHaveBeenCalled();
});
