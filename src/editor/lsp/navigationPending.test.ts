import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import type { commands } from "@/platform/ipc";
import { useProject } from "@/platform/project";

const lspGoto = vi.hoisted(() => vi.fn<typeof commands.lspGoto>());
const fsReadText = vi.hoisted(() => vi.fn<typeof commands.fsReadText>());
vi.mock("@/platform/ipc", async (orig) => {
  const real = await orig<typeof import("@/platform/ipc")>();
  return { ...real, isTauri: () => true, commands: { ...real.commands, lspGoto, fsReadText } };
});

const { gotoTargets } = await import("./navigation");
const { lspDoc } = await import("./lsp");
const { useEditorRegistry } = await import("@/editor/api");
const { useLspPopup, closePopup } = await import("./popupStore");
const { provideEditorService } = await import("@/workbench/contributions");
const openFile = vi.fn(async (_path: string, _options?: unknown) => {});
provideEditorService({ openFile } as never);

let view: EditorView;
beforeEach(() => {
  vi.useFakeTimers();
  openFile.mockClear();
  fsReadText.mockReset().mockResolvedValue({ status: "ok", data: "" });
  useProject.setState({ project: null });
  useEditorRegistry.setState({ active: null });
  closePopup();
  view = new EditorView({
    state: EditorState.create({
      doc: "run()",
      extensions: [lspDoc.of({ doc: 1, path: "/p/main.rs", language: "rust", uiMode: "Rust", readOnly: false })],
    }),
    parent: document.body,
  });
});
afterEach(() => {
  view.destroy();
  useProject.setState({ project: null });
  useEditorRegistry.setState({ active: null });
  closePopup();
  vi.useRealTimers();
});

describe("slow navigation feedback", () => {
  it("shows progress while finding a definition, then opens the target and clears the hint", async () => {
    let resolve!: (result: Awaited<ReturnType<typeof commands.lspGoto>>) => void;
    lspGoto.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
    const pending = gotoTargets(view, "definition");
    await vi.advanceTimersByTimeAsync(250);
    expect(document.querySelector(".cm-lsp-hint")?.textContent).toBe("Finding navigation target…");
    expect(openFile).not.toHaveBeenCalled();
    resolve({
      status: "ok",
      data: [{ path: "/p/lib.rs", range: { start: { line: 3, col: 4 }, end: { line: 3, col: 7 } }, uri: null }],
    });
    await pending;
    expect(openFile).toHaveBeenCalledWith("/p/lib.rs", { selection: { line: 3, col: 4 }, focus: true });
    expect(document.querySelector(".cm-lsp-hint")).toBeNull();
  });

  it("replaces progress with the server error", async () => {
    let resolve!: (result: Awaited<ReturnType<typeof commands.lspGoto>>) => void;
    lspGoto.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
    const pending = gotoTargets(view, "implementation");
    await vi.advanceTimersByTimeAsync(250);
    resolve({ status: "error", error: { kind: "protocol", message: "Server timed out" } });
    await pending;
    expect(document.querySelector(".cm-lsp-hint")?.textContent).toBe("Server timed out");
    expect(openFile).not.toHaveBeenCalled();
  });
});

describe("pending navigation ownership", () => {
  for (const change of ["source", "caret", "document", "project", "closed view", "active editor", "Escape"] as const) {
    it(`ignores a successful result after changing the ${change}`, async () => {
      let resolve!: (result: Awaited<ReturnType<typeof commands.lspGoto>>) => void;
      lspGoto.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
      if (change === "active editor") useEditorRegistry.setState({ active: { view } as never });
      const pending = gotoTargets(view, "definition");
      await vi.advanceTimersByTimeAsync(250);
      if (change === "source") view.dispatch({ changes: { from: 0, to: 3, insert: "other" } });
      if (change === "caret") view.dispatch({ selection: { anchor: 3 } });
      if (change === "document")
        view.setState(
          EditorState.create({
            doc: "run()",
            extensions: [lspDoc.of({ doc: 2, path: "/p/other.rs", language: "rust", uiMode: "Rust", readOnly: false })],
          }),
        );
      if (change === "project") useProject.setState({ project: { root: "/other" } as never });
      if (change === "closed view") view.destroy();
      if (change === "active editor") useEditorRegistry.setState({ active: null });
      if (change === "Escape") view.dom.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
      resolve({
        status: "ok",
        data: [{ path: "/p/old.rs", range: { start: { line: 1, col: 0 }, end: { line: 1, col: 3 } }, uri: null }],
      });
      await pending;
      expect(openFile).not.toHaveBeenCalled();
      expect(document.querySelector(".cm-lsp-hint")).toBeNull();
    });
  }

  it("ignores a stale error after typing", async () => {
    let resolve!: (result: Awaited<ReturnType<typeof commands.lspGoto>>) => void;
    lspGoto.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
    const pending = gotoTargets(view, "implementation");
    await vi.advanceTimersByTimeAsync(250);
    view.dispatch({ changes: { from: 0, to: 3, insert: "other" } });
    resolve({ status: "error", error: { kind: "protocol", message: "Old server error" } });
    await pending;
    expect(document.querySelector(".cm-lsp-hint")).toBeNull();
  });

  it("allows the newest lookup and ignores an older successful reply", async () => {
    let resolve!: (result: Awaited<ReturnType<typeof commands.lspGoto>>) => void;
    lspGoto.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
    const old = gotoTargets(view, "definition");
    await vi.advanceTimersByTimeAsync(250);
    lspGoto.mockResolvedValueOnce({
      status: "ok",
      data: [{ path: "/p/new.rs", range: { start: { line: 2, col: 0 }, end: { line: 2, col: 3 } }, uri: null }],
    });
    await gotoTargets(view, "implementation");
    resolve({
      status: "ok",
      data: [{ path: "/p/old.rs", range: { start: { line: 1, col: 0 }, end: { line: 1, col: 3 } }, uri: null }],
    });
    await old;
    expect(openFile.mock.calls.map(([path]) => path)).toEqual(["/p/new.rs"]);
  });
});

for (const changed of [false, true]) {
  it(`checks ownership again when choosing a prepared target (changed=${changed})`, async () => {
    lspGoto.mockResolvedValueOnce({
      status: "ok",
      data: [
        { path: "/p/a.rs", range: { start: { line: 1, col: 0 }, end: { line: 1, col: 3 } }, uri: null },
        { path: "/p/b.rs", range: { start: { line: 2, col: 0 }, end: { line: 2, col: 3 } }, uri: null },
      ],
    });
    await gotoTargets(view, "definition");
    const popup = useLspPopup.getState().popup;
    if (popup?.kind !== "list") throw new Error("expected targets chooser");
    const row = popup.items[1]!;
    if (changed) view.dispatch({ changes: { from: 0, to: 3, insert: "other" } });
    // The real picker closes before invoking the accepted row.
    closePopup();
    await row.run();
    expect(openFile.mock.calls.map(([path]) => path)).toEqual(changed ? [] : ["/p/b.rs"]);
  });
}

it("Escape before progress appears cancels the delayed hint as well as the eventual target", async () => {
  let resolve!: (result: Awaited<ReturnType<typeof commands.lspGoto>>) => void;
  lspGoto.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
  const pending = gotoTargets(view, "definition");
  await vi.advanceTimersByTimeAsync(0);
  expect(resolve).toBeTypeOf("function");
  view.dom.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  await vi.advanceTimersByTimeAsync(250);
  expect(document.querySelector(".cm-lsp-hint")).toBeNull();
  resolve({
    status: "ok",
    data: [{ path: "/p/old.rs", range: { start: { line: 1, col: 0 }, end: { line: 1, col: 3 } }, uri: null }],
  });
  await pending;
  expect(openFile).not.toHaveBeenCalled();
});

it("ignores prepared locations if source changes while preview files are read", async () => {
  let resolve!: (result: Awaited<ReturnType<typeof commands.fsReadText>>) => void;
  fsReadText.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
  lspGoto.mockResolvedValueOnce({
    status: "ok",
    data: [
      { path: "/p/a.rs", range: { start: { line: 1, col: 0 }, end: { line: 1, col: 3 } }, uri: null },
      { path: "/p/b.rs", range: { start: { line: 2, col: 0 }, end: { line: 2, col: 3 } }, uri: null },
    ],
  });
  const pending = gotoTargets(view, "definition");
  await vi.advanceTimersByTimeAsync(0);
  expect(resolve).toBeTypeOf("function");
  view.dispatch({ changes: { from: 0, to: 3, insert: "other" } });
  resolve({ status: "ok", data: "run()" });
  await pending;
  expect(useLspPopup.getState().popup).toBeNull();
  expect(openFile).not.toHaveBeenCalled();
});
