import { afterEach, describe, expect, it, vi } from "vitest";
import { ask } from "@/workbench/messages";
vi.mock("@/workbench/messages", () => ({ ask: vi.fn() }));
import { useProject } from "@/platform/project";
import { createMemoryBackend } from "./backend";
import { EditorController, openFilePaths, useTabs } from "./controller";
import { initialTabsState } from "./model/tabs";
import { useDocMeta } from "./model/document";

let ctrl: EditorController | undefined;
async function settle() {
  for (let i = 0; i < 15; i++) await new Promise((resolve) => setTimeout(resolve, 0));
}
async function setup(autoSave = false) {
  useProject.setState({ settings: { files: { autoSave } } as never });
  const backend = createMemoryBackend({ "/p/a.txt": "original\n", "/p/b.txt": "second\n" });
  ctrl = new EditorController(backend, (fn) => queueMicrotask(fn));
  ctrl.start();
  await ctrl.openFile("/p/a.txt", { focus: false });
  const pane = useTabs.getState().panes[useTabs.getState().activePane]!;
  ctrl.slotFor(pane.active!)!.dispatchLocal({ changes: { from: 0, to: 8, insert: "local" } });
  await settle();
  return { backend, pane: pane.id, tab: pane.active!, controller: ctrl };
}
afterEach(() => {
  ctrl?.stop();
  ctrl = undefined;
  useTabs.setState(initialTabsState(), true);
  useDocMeta.setState({}, true);
  useProject.setState({ settings: null });
  vi.mocked(ask).mockReset();
});

describe("modified tab close protection", () => {
  it("keeps the last modified tab and buffer when Cancel is chosen", async () => {
    const { controller, backend, pane, tab } = await setup();
    vi.mocked(ask).mockResolvedValue({ button: "cancel", checked: false });
    controller.closeTab(pane, tab);
    await settle();
    expect(ask).toHaveBeenCalledOnce();
    expect(openFilePaths(useTabs.getState())).toEqual(["/p/a.txt"]);
    expect(controller.manager.get("/p/a.txt")?.sync.localText.toString()).toBe("local\n");
    expect(backend.files.get("/p/a.txt")).toBe("original\n");
  });

  it("closes only after explicit discard and leaves disk unchanged", async () => {
    const { controller, backend, pane, tab } = await setup();
    vi.mocked(ask).mockResolvedValue({ button: "discard", checked: false });
    controller.closeTab(pane, tab);
    await settle();
    expect(ask).toHaveBeenCalledOnce();
    expect(openFilePaths(useTabs.getState())).toEqual([]);
    expect(controller.manager.get("/p/a.txt")).toBeUndefined();
    expect(backend.files.get("/p/a.txt")).toBe("original\n");
  });

  it("saves the modified document before closing", async () => {
    const { controller, backend, pane, tab } = await setup();
    vi.mocked(ask).mockResolvedValue({ button: "save", checked: false });
    controller.closeTab(pane, tab);
    await settle();
    expect(ask).toHaveBeenCalledOnce();
    expect(openFilePaths(useTabs.getState())).toEqual([]);
    expect(backend.files.get("/p/a.txt")).toBe("local\n");
  });

  it("keeps the tab visible if external-overwrite confirmation is cancelled", async () => {
    const { controller, backend, pane, tab } = await setup();
    backend.files.set("/p/a.txt", "external\n");
    vi.mocked(ask)
      .mockResolvedValueOnce({ button: "save", checked: false })
      .mockResolvedValueOnce({ button: "cancel", checked: false });
    controller.closeTab(pane, tab);
    await settle();
    expect(ask).toHaveBeenCalledTimes(2);
    expect(openFilePaths(useTabs.getState())).toEqual(["/p/a.txt"]);
    expect(backend.files.get("/p/a.txt")).toBe("external\n");
  });

  it("does not ask when another split still holds the modified document", async () => {
    const { controller, pane, tab } = await setup();
    controller.openInOpposite(pane, tab);
    await settle();
    controller.closeTab(pane, tab);
    await settle();
    expect(ask).not.toHaveBeenCalled();
    expect(openFilePaths(useTabs.getState())).toEqual(["/p/a.txt"]);
    expect(controller.manager.get("/p/a.txt")?.sync.localText.toString()).toBe("local\n");
  });

  it("asks once for a bulk close and leaves all tabs on Cancel", async () => {
    const { controller, backend, pane } = await setup();
    await controller.openFile("/p/b.txt", { focus: false });
    const second = useTabs.getState().panes[pane]!.active!;
    controller.slotFor(second)!.dispatchLocal({ changes: { from: 0, to: 6, insert: "changed" } });
    await settle();
    vi.mocked(ask).mockResolvedValue({ button: "cancel", checked: false });
    controller.closeAll(pane);
    await settle();
    expect(ask).toHaveBeenCalledOnce();
    expect(openFilePaths(useTabs.getState())).toEqual(["/p/a.txt", "/p/b.txt"]);
    expect(backend.files.get("/p/a.txt")).toBe("original\n");
    expect(backend.files.get("/p/b.txt")).toBe("second\n");
  });

  it("does not discard edits made while the close question is open", async () => {
    const { controller, pane, tab } = await setup();
    let answer!: (value: { button: string; checked: boolean }) => void;
    vi.mocked(ask).mockImplementation(
      () =>
        new Promise((resolve) => {
          answer = resolve;
        }),
    );
    controller.closeTab(pane, tab);
    await vi.waitFor(() => expect(ask).toHaveBeenCalledOnce());
    controller.slotFor(tab)!.dispatchLocal({ changes: { from: 0, insert: "new " } });
    answer({ button: "discard", checked: false });
    await settle();
    expect(openFilePaths(useTabs.getState())).toEqual(["/p/a.txt"]);
    expect(controller.manager.get("/p/a.txt")?.sync.localText.toString()).toBe("new local\n");
  });

  it("can explicitly discard a previously cancelled save conflict", async () => {
    const { controller, backend, pane, tab } = await setup();
    backend.files.set("/p/a.txt", "external\n");
    vi.mocked(ask).mockResolvedValue({ button: "cancel", checked: false });
    await expect(controller.manager.get("/p/a.txt")!.save("explicit")).rejects.toMatchObject({ kind: "cancelled" });
    vi.mocked(ask).mockResolvedValue({ button: "discard", checked: false });
    controller.closeTab(pane, tab);
    await settle();
    expect(openFilePaths(useTabs.getState())).toEqual([]);
    expect(controller.manager.get("/p/a.txt")).toBeUndefined();
    expect(backend.files.get("/p/a.txt")).toBe("external\n");
  });

  it("saves every modified file before Close All removes its tabs", async () => {
    const { controller, backend, pane } = await setup();
    await controller.openFile("/p/b.txt", { focus: false });
    controller
      .slotFor(useTabs.getState().panes[pane]!.active!)!
      .dispatchLocal({ changes: { from: 0, to: 6, insert: "changed" } });
    await settle();
    vi.mocked(ask).mockResolvedValue({ button: "save", checked: false });
    controller.closeAll(pane);
    await settle();
    expect(ask).toHaveBeenCalledOnce();
    expect(openFilePaths(useTabs.getState())).toEqual([]);
    expect(backend.files.get("/p/a.txt")).toBe("local\n");
    expect(backend.files.get("/p/b.txt")).toBe("changed\n");
  });

  it("does not close a new tab opened while the bulk question is pending", async () => {
    const { controller } = await setup();
    let answer!: (value: { button: string; checked: boolean }) => void;
    vi.mocked(ask).mockImplementation(
      () =>
        new Promise((resolve) => {
          answer = resolve;
        }),
    );
    controller.closeEverything();
    await controller.openFile("/p/b.txt", { focus: false });
    answer({ button: "discard", checked: false });
    await settle();
    expect(openFilePaths(useTabs.getState())).toEqual(["/p/a.txt", "/p/b.txt"]);
  });

  it("preserves default autosave on tab close without a new prompt", async () => {
    const { controller, backend, pane, tab } = await setup(true);
    controller.closeTab(pane, tab);
    await settle();
    expect(ask).not.toHaveBeenCalled();
    expect(openFilePaths(useTabs.getState())).toEqual([]);
    expect(backend.files.get("/p/a.txt")).toBe("local\n");
  });
});
