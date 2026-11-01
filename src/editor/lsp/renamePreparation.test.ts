import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { EditorState, StateEffect } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

const mock = vi.hoisted(() => ({ prepare: vi.fn(), legacy: vi.fn(), rename: vi.fn(), highlights: vi.fn() }));
vi.mock("@/platform/ipc", async (original) => {
  const actual = await original<typeof import("@/platform/ipc")>();
  return {
    ...actual,
    isTauri: () => true,
    commands: {
      ...actual.commands,
      lspPrepareRenameDetails: mock.prepare,
      lspPrepareRename: mock.legacy,
      lspRename: mock.rename,
      lspDocumentHighlights: mock.highlights,
    },
  };
});
const { startRename } = await import("./rename");
const { lspDoc } = await import("./lsp");
const { inplaceRenameExtension, inplaceSession } = await import("./inplaceRename");
const { closePopup, useLspPopup } = await import("./popupStore");
const text = "😀 Routes::get('/invoices')->name('invoice');";
const from = text.lastIndexOf("invoice");
const range = { start: { line: 0, col: from }, end: { line: 0, col: from + 7 } };
const context = { doc: 7, path: "/p/routes/web.php", language: "php", uiMode: null, readOnly: false };
const ok = (data: unknown) => ({ status: "ok", data });
let view: EditorView;
beforeEach(() => {
  vi.resetAllMocks();
  closePopup();
  mock.prepare.mockResolvedValue(ok({ range, placeholder: "openstorm.billing.invoice" }));
  mock.legacy.mockResolvedValue(ok(range));
  mock.highlights.mockResolvedValue(ok([]));
  mock.rename.mockResolvedValue(
    ok({
      edit: {
        files: [
          { kind: "edit", path: context.path, edits: [{ range, new_text: "details" }] },
          { kind: "edit", path: "/p/caller.php", edits: [{ range, new_text: "openstorm.billing.details" }] },
        ],
        guard: null,
      },
      incomplete: null,
    }),
  );
  view = new EditorView({
    state: EditorState.create({
      doc: text,
      selection: { anchor: from + 2 },
      extensions: [lspDoc.of(context), inplaceRenameExtension()],
    }),
    parent: document.body,
  });
});
afterEach(() => {
  closePopup();
  view.destroy();
});

it("uses a different semantic placeholder in the existing dialog without inserting it into the short declaration", async () => {
  await startRename(view);
  const popup = useLspPopup.getState().popup;
  expect(popup?.kind).toBe("rename");
  if (popup?.kind !== "rename") return;
  expect(popup.initial).toBe("openstorm.billing.invoice");
  expect(popup.title).toContain("openstorm.billing.invoice");
  expect(inplaceSession(view.state)).toBeNull();
  expect(view.state.doc.toString()).toBe(text);
  popup.onSubmit("openstorm.billing.details");
  await vi.waitFor(() =>
    expect(mock.rename).toHaveBeenCalledWith(
      expect.any(Number),
      7,
      { line: 0, col: from + 2 },
      "openstorm.billing.details",
    ),
  );
  await vi.waitFor(() => expect(useLspPopup.getState().popup?.kind).toBe("preview"));
  expect(view.state.doc.toString()).toBe(text);
});

it.each([null, "invoice"])("retains normal in-place rename when the placeholder is %s", async (placeholder) => {
  mock.prepare.mockResolvedValue(ok({ range, placeholder }));
  await startRename(view);
  expect(inplaceSession(view.state)).not.toBeNull();
  expect(view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to)).toBe("invoice");
  expect(useLspPopup.getState().popup).toBeNull();
  view.dispatch({ changes: { from, to: from + 7, insert: "updated" }, userEvent: "input.type" });
  await startRename(view);
  const popup = useLspPopup.getState().popup;
  expect(popup?.kind).toBe("rename");
  if (popup?.kind !== "rename") return;
  expect(popup.initial).toBe("updated");
  expect(view.state.doc.toString()).toBe(text);
  popup.onSubmit("updated");
  await vi.waitFor(() =>
    expect(mock.rename).toHaveBeenCalledWith(expect.any(Number), 7, { line: 0, col: from + 2 }, "updated"),
  );
  await vi.waitFor(() => expect(useLspPopup.getState().popup?.kind).toBe("preview"));
});

it.each(["typed", "replaced", "read-only"])("discards late preparation after the editor is %s", async (mode) => {
  let resolve!: (value: unknown) => void;
  mock.prepare.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = startRename(view);
  await vi.waitFor(() => expect(mock.prepare).toHaveBeenCalledOnce());
  if (mode === "typed") view.dispatch({ changes: { from, to: from + 7, insert: "another" } });
  else
    view.dispatch({
      effects: StateEffect.reconfigure.of([
        lspDoc.of(mode === "replaced" ? { ...context, doc: 8, path: "/p/other.php" } : { ...context, readOnly: true }),
        inplaceRenameExtension(),
      ]),
    });
  resolve(ok({ range, placeholder: "openstorm.billing.invoice" }));
  await pending;
  expect(useLspPopup.getState().popup).toBeNull();
  expect(inplaceSession(view.state)).toBeNull();
  expect(mock.rename).not.toHaveBeenCalled();
});

it("refuses a dialog submission after the displayed source changed", async () => {
  await startRename(view);
  const popup = useLspPopup.getState().popup;
  expect(popup?.kind).toBe("rename");
  if (popup?.kind !== "rename") return;
  view.dispatch({ changes: { from, to: from + 7, insert: "another" } });
  popup.onSubmit("openstorm.billing.details");
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(mock.rename).not.toHaveBeenCalled();
  expect(view.state.doc.toString()).toContain("name('another')");
});

it("drops a completed rename plan after the source changes while the request is running", async () => {
  await startRename(view);
  const popup = useLspPopup.getState().popup;
  if (popup?.kind !== "rename") throw new Error("rename dialog missing");
  let resolve!: (value: unknown) => void;
  mock.rename.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  popup.onSubmit("openstorm.billing.details");
  await vi.waitFor(() => expect(mock.rename).toHaveBeenCalledOnce());
  view.dispatch({ changes: { from, to: from + 7, insert: "another" } });
  resolve(
    ok({
      edit: {
        files: [
          { kind: "edit", path: context.path, edits: [{ range, new_text: "details" }] },
          { kind: "edit", path: "/p/caller.php", edits: [{ range, new_text: "openstorm.billing.details" }] },
        ],
        guard: null,
      },
      incomplete: null,
    }),
  );
  await new Promise((done) => setTimeout(done, 0));
  expect(useLspPopup.getState().popup?.kind).not.toBe("preview");
  expect(view.state.doc.toString()).toContain("name('another')");
});
