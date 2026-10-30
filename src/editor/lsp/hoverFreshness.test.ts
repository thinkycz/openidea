import { afterEach, describe, expect, it, vi } from "vitest";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import type { Hover } from "@/platform/ipc";

const cmd = {
  lspHover: vi.fn((): Promise<{ status: "ok"; data: Hover | null }> => Promise.resolve({ status: "ok", data: null })),
  cancelRequest: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
};
vi.mock("@/platform/ipc", async (orig) => {
  const actual = await orig<typeof import("@/platform/ipc")>();
  return { ...actual, isTauri: () => true, commands: { ...actual.commands, ...cmd } };
});
const { lspDoc } = await import("./lsp");
const { requestHover, quickDocumentation, hoverExtension } = await import("./hover");
const { closePopup, useLspPopup } = await import("./popupStore");
const views: EditorView[] = [];
const context = { doc: 1, path: "/fixture/main.php", language: "php", uiMode: "PHP", readOnly: false };
const hover: Hover = { contents: { kind: "markdown", value: "current rules" }, range: null };

function editor() {
  const facet = new Compartment();
  const view = new EditorView({
    state: EditorState.create({ doc: "field", extensions: [facet.of(lspDoc.of(context)), hoverExtension()] }),
    parent: document.body.appendChild(document.createElement("div")),
  });
  views.push(view);
  return { view, facet };
}
function response() {
  let resolve!: (value: { status: "ok"; data: Hover | null }) => void;
  cmd.lspHover.mockReturnValueOnce(new Promise((r) => (resolve = r)));
  return (data: Hover | null) => resolve({ status: "ok", data });
}
afterEach(() => {
  for (const view of views.splice(0)) view.destroy();
  closePopup();
  document.body.replaceChildren();
  vi.clearAllMocks();
});

describe("documentation response freshness", () => {
  it("Escape cancels pending documentation without a late popup", async () => {
    const { view } = editor();
    const resolve = response();
    const pending = quickDocumentation(view);
    await vi.waitFor(() => expect(cmd.lspHover).toHaveBeenCalledOnce());
    view.contentDOM.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    resolve(hover);
    await pending;
    expect(useLspPopup.getState().popup).toBeNull();
    expect(cmd.cancelRequest).toHaveBeenCalledOnce();
  });
  it("accepts an unchanged editor's response", async () => {
    const { view } = editor();
    const resolve = response();
    const result = requestHover(view, 2);
    await vi.waitFor(() => expect(cmd.lspHover).toHaveBeenCalledOnce());
    resolve(hover);
    expect(await result).toEqual(hover);
  });

  it.each(["typing", "document switch", "file move", "view closed"])(
    "rejects a late response after %s",
    async (mode) => {
      const { view, facet } = editor();
      const resolve = response();
      const result = requestHover(view, 2);
      const rejected = expect(result).rejects.toMatchObject({ kind: "cancelled" });
      await vi.waitFor(() => expect(cmd.lspHover).toHaveBeenCalledOnce());
      if (mode === "typing") view.dispatch({ changes: { from: 0, to: 5, insert: "different field" } });
      else if (mode === "view closed") view.destroy();
      else
        view.dispatch({
          effects: facet.reconfigure(lspDoc.of({ ...context, doc: mode === "file move" ? 1 : 2, path: "/other.php" })),
        });
      resolve(hover);
      await rejected;
    },
  );

  it("keeps only the latest requested hover", async () => {
    const { view } = editor();
    const oldResponse = response();
    const old = requestHover(view, 1);
    const rejected = expect(old).rejects.toMatchObject({ kind: "cancelled" });
    await vi.waitFor(() => expect(cmd.lspHover).toHaveBeenCalledOnce());
    const currentResponse = response();
    const current = requestHover(view, 3);
    await vi.waitFor(() => expect(cmd.lspHover).toHaveBeenCalledTimes(2));
    currentResponse(hover);
    expect(await current).toEqual(hover);
    oldResponse({ contents: { kind: "markdown", value: "outdated" }, range: null });
    await rejected;
  });

  it("does not report missing documentation for a discarded empty response", async () => {
    const { view } = editor();
    const resolve = response();
    const result = quickDocumentation(view);
    await vi.waitFor(() => expect(cmd.lspHover).toHaveBeenCalledOnce());
    view.dispatch({ changes: { from: 0, to: 5, insert: "changed" } });
    resolve(null);
    await result;
    expect(useLspPopup.getState().popup).toBeNull();
    expect(document.querySelector(".cm-lsp-hint")).toBeNull();
  });
});
