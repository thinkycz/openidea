import { afterEach, expect, it, vi } from "vitest";
import { useProject } from "@/platform/project";
import { onEditorEvent } from "@/editor/api";
import type { DocumentInfo } from "@/platform/ipc";
import { createMemoryBackend } from "./backend";
import { EditorController, useTabs } from "./controller";
import { DocModel, DocumentManager, useDocMeta } from "./model/document";
import { initialTabsState } from "./model/tabs";

let controller: EditorController | undefined;
afterEach(() => {
  controller?.stop();
  controller = undefined;
  useTabs.setState(initialTabsState(), true);
  useDocMeta.setState({}, true);
  useProject.setState({ project: null, settings: null, lightEdit: false });
  localStorage.clear();
});

it("does not resurrect a discarded conflicted model after project close and reopen", async () => {
  const project = { root: "/p", name: "p" } as never;
  useProject.setState({ project, settings: { files: { autoSave: false } } as never });
  const backend = createMemoryBackend({ "/p/a.txt": "original\n" });
  controller = new EditorController(backend, (fn) => queueMicrotask(fn));
  controller.start();
  await controller.openFile("/p/a.txt", { focus: false });
  const state = useTabs.getState();
  const slot = controller.slotFor(state.panes[state.activePane]!.active!)!;
  slot.dispatchLocal({ changes: { from: 0, to: 8, insert: "discarded" } });
  await slot.model.sync.whenIdle();
  backend.files.set("/p/a.txt", "external\n");
  await slot.model.save("auto");
  expect(slot.model.hasSaveConflict()).toBe(true);

  // The host replaces its documents after the user allows project closing.
  await backend.close(slot.model.doc);
  useProject.setState({ project: null });
  useProject.setState({ project });
  await controller.openFile("/p/a.txt", { focus: false });
  const reopened = controller.manager.get("/p/a.txt")!;
  expect(reopened.sync.localText.toString()).toBe("external\n");
  expect(reopened.doc).not.toBe(slot.model.doc);
  expect(reopened.getMeta().dirty).toBe(false);
  expect(backend.files.get("/p/a.txt")).toBe("external\n");
});

it("rejects an old project's late open without deleting the new project's pending open", async () => {
  const backend = createMemoryBackend({ "/p/a.txt": "external\n" });
  const open = backend.open.bind(backend);
  const pending: Array<{ info: DocumentInfo; resolve: (info: DocumentInfo) => void }> = [];
  backend.open = vi.fn(async (path) => {
    const info = await open(path);
    return new Promise<DocumentInfo>((resolve) => pending.push({ info, resolve }));
  });
  const manager = new DocumentManager(backend, { settings: () => null, notify: () => {} });
  try {
    const old = manager.acquire("/p/a.txt");
    await vi.waitFor(() => expect(pending).toHaveLength(1));
    await backend.close(pending[0]!.info.id);
    manager.resetProject();
    const fresh = manager.acquire("/p/a.txt");
    await vi.waitFor(() => expect(pending).toHaveLength(2));
    pending[0]!.resolve(pending[0]!.info);
    await expect(old).rejects.toMatchObject({ kind: "cancelled" });
    const same = manager.acquire("/p/a.txt");
    expect(backend.open).toHaveBeenCalledTimes(2);
    pending[1]!.resolve(pending[1]!.info);
    const [model, reused] = await Promise.all([fresh, same]);
    expect(reused).toBe(model);
    expect(manager.get("/p/a.txt")).toBe(model);
    expect(model.doc).toBe(pending[1]!.info.id);
  } finally {
    manager.resetProject();
    manager.stop();
  }
});

it("lets a close-event subscriber acquire the new project's model instead of a disposed one", async () => {
  const backend = createMemoryBackend({ "/p/a.txt": "external\n" });
  const manager = new DocumentManager(backend, { settings: () => null, notify: () => {} });
  const old = await manager.acquire("/p/a.txt");
  await backend.close(old.doc);
  let opened: ReturnType<DocumentManager["acquire"]> | undefined;
  const off = onEditorEvent((event) => {
    if (event.type === "docClosed") opened = manager.acquire(event.path);
  });
  try {
    manager.resetProject();
    const fresh = await opened!;
    expect(fresh.doc).not.toBe(old.doc);
    expect(fresh.closed).toBe(false);
    expect(manager.get("/p/a.txt")).toBe(fresh);
  } finally {
    off();
    manager.resetProject();
    manager.stop();
  }
});

it("rejects a previous project's model after lazy language loading finishes", async () => {
  const backend = createMemoryBackend({ "/p/a.txt": "external\n" });
  const manager = new DocumentManager(backend, { settings: () => null, notify: () => {} });
  let finish!: () => void;
  const gate = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const load = vi.spyOn(DocModel.prototype, "loadLanguage").mockImplementationOnce(() => gate);
  try {
    const old = manager.acquire("/p/a.txt");
    await vi.waitFor(() => expect(load).toHaveBeenCalledOnce());
    const oldId = useDocMeta.getState()["/p/a.txt"]!.doc;
    await backend.close(oldId);
    manager.resetProject();
    const fresh = await manager.acquire("/p/a.txt");
    finish();
    await expect(old).rejects.toMatchObject({ kind: "cancelled" });
    expect(manager.get("/p/a.txt")).toBe(fresh);
    expect(useDocMeta.getState()["/p/a.txt"]!.doc).toBe(fresh.doc);
    expect(fresh.closed).toBe(false);
  } finally {
    finish();
    load.mockRestore();
    manager.resetProject();
    manager.stop();
  }
});
