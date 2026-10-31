import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useProject } from "@/platform/project";
import { ask } from "@/workbench/messages";
import { createMemoryBackend } from "./backend";
import { EditorController, useTabs } from "./controller";
import { initialTabsState } from "./model/tabs";
import { useDocMeta } from "./model/document";

type FocusListener = (event: { payload: boolean }) => void;
const native = vi.hoisted(() => ({ listeners: new Set<FocusListener>(), subscribe: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ onFocusChanged: native.subscribe }),
}));
vi.mock("@/platform/ipc", async (original) => ({
  ...(await original<typeof import("@/platform/ipc")>()),
  isTauri: () => true,
}));
vi.mock("@/workbench/messages", () => ({ ask: vi.fn() }));

let controller: EditorController | undefined;
beforeEach(() => {
  native.subscribe.mockImplementation(async (listener: FocusListener) => {
    native.listeners.add(listener);
    return () => native.listeners.delete(listener);
  });
});
afterEach(() => {
  controller?.stop();
  controller = undefined;
  native.listeners.clear();
  native.subscribe.mockReset();
  vi.mocked(ask).mockReset();
  useTabs.setState(initialTabsState(), true);
  useDocMeta.setState({}, true);
  useProject.setState({ settings: null });
});

function focus(focused: boolean) {
  for (const listener of native.listeners) listener({ payload: focused });
}

async function setup(autoSave = true) {
  useProject.setState({ settings: { files: { autoSave } } as never });
  const backend = createMemoryBackend({ "/p/a.txt": "original\n" });
  controller = new EditorController(backend, (fn) => queueMicrotask(fn));
  controller.start();
  await controller.openFile("/p/a.txt", { focus: false });
  const state = useTabs.getState();
  const slot = controller.slotFor(state.panes[state.activePane]!.active!)!;
  slot.dispatchLocal({ changes: { from: 0, to: 8, insert: "local" } });
  await slot.model.sync.whenIdle();
  return { backend, model: slot.model };
}

describe("native window autosave", () => {
  it("saves dirty buffers on native focus loss without a DOM blur event", async () => {
    const { backend } = await setup();
    focus(true);
    expect(backend.files.get("/p/a.txt")).toBe("original\n");
    await vi.waitFor(() => {
      focus(false);
      expect(backend.files.get("/p/a.txt")).toBe("local\n");
    });
  });

  it("keeps manual-save buffers unsaved on native focus loss", async () => {
    const { backend } = await setup(false);
    await vi.waitFor(() => expect(native.subscribe).toHaveBeenCalledOnce());
    focus(false);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(backend.files.get("/p/a.txt")).toBe("original\n");
  });

  it("pauses autosave at an external conflict without asking or overwriting", async () => {
    const { backend, model } = await setup();
    backend.files.set("/p/a.txt", "external\n");
    await vi.waitFor(() => {
      focus(false);
      expect(model.hasSaveConflict()).toBe(true);
    });
    expect(backend.files.get("/p/a.txt")).toBe("external\n");
    expect(model.sync.localText.toString()).toBe("local\n");
    expect(ask).not.toHaveBeenCalled();
  });

  it("releases a native subscription that finishes after the controller stops", async () => {
    let resolve!: (off: () => void) => void;
    const pending = new Promise<() => void>((done) => (resolve = done));
    native.subscribe.mockReturnValue(pending);
    await setup();
    await vi.waitFor(() => expect(native.subscribe).toHaveBeenCalledOnce());
    controller!.stop();
    controller = undefined;
    const off = vi.fn();
    resolve(off);
    await vi.waitFor(() => expect(off).toHaveBeenCalledOnce());
  });
});
