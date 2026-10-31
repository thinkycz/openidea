import { afterEach, describe, expect, it, vi } from "vitest";
import { registerAction } from "@/platform/actions";
import { installSettingsHistoryMenu } from "../window/bridge";
const mock = vi.hoisted(() => ({
  label: "settings-main",
  handlers: new Map<string, (e: { payload: unknown }) => void>(),
  off: vi.fn(),
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    label: mock.label,
    listen: async (event: string, handler: (e: { payload: unknown }) => void) => {
      mock.handlers.set(event, handler);
      return () => {
        mock.handlers.delete(event);
        mock.off();
      };
    },
  }),
}));
vi.mock("@/platform/ipc", async (orig) => ({
  ...(await orig<typeof import("@/platform/ipc")>()),
  isTauri: () => true,
}));
afterEach(() => {
  mock.handlers.clear();
  mock.label = "settings-main";
  mock.off.mockClear();
});

describe("modal Settings history menu", () => {
  it("edits the focused local input and never forwards history to a project action", async () => {
    const descriptor = Object.getOwnPropertyDescriptor(document, "execCommand");
    const exec = vi.fn();
    const parentUndo = vi.fn();
    const removeAction = registerAction({ id: "edit.undo", title: "Undo", run: parentUndo });
    const input = document.createElement("input");
    document.body.append(input);
    let stop = () => {};
    try {
      Object.defineProperty(document, "execCommand", { configurable: true, value: exec });
      input.focus();
      stop = installSettingsHistoryMenu();
      await Promise.resolve();
      const listener = mock.handlers.get("openidea://menu");
      expect(listener).toBeDefined();
      for (const payload of [{ action: "edit.undo" }, { action: "edit.redo" }, { action: "file.save" }, null])
        listener!({ payload });
      expect(exec.mock.calls).toEqual([["undo"], ["redo"]]);
      expect(parentUndo).not.toHaveBeenCalled();
    } finally {
      stop();
      input.remove();
      removeAction();
      if (descriptor) Object.defineProperty(document, "execCommand", descriptor);
      else Reflect.deleteProperty(document, "execCommand");
    }
    expect(mock.off).toHaveBeenCalledOnce();
  });
  it("does not install a Settings handler in a project window", () => {
    mock.label = "main";
    const stop = installSettingsHistoryMenu();
    expect(mock.handlers.size).toBe(0);
    stop();
  });
});
