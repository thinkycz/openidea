import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EditorState, StateEffect } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import type { ProjectInfo, Settings, TextEdit } from "@/bindings";
import { documentSynced, registerEditorContribution, registerPendingEdits, type EditorDocContext } from "@/editor/api";
import { createMemoryBackend } from "@/editor/core/backend";
import { DocumentManager, useDocMeta } from "@/editor/core/model/document";
import { EditorSlot, type SlotHost } from "@/editor/core/model/slot";
import { formatIndentFacet } from "@/editor/core/cm/setup";
import { commands, IpcError } from "@/platform/ipc";
import { registerAction } from "@/platform/actions";
import { useProject } from "@/platform/project";
import * as tauri from "@/platform/tauriCore";
import { formatDocument, formatLines } from "./format";
import * as lsp from "./lsp";

const source = "<?php /* 😀 */ echo 1;";
const formatted = "<?php /* 😀 */\necho 1;\n";
const context: EditorDocContext = { doc: 1, path: "/project/old.php", language: "php", uiMode: null, readOnly: false };
const edits: TextEdit[] = [
  { range: { start: { line: 0, col: 0 }, end: { line: 0, col: source.length } }, new_text: formatted },
];
const views: EditorView[] = [];
let previous = useProject.getState();

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
function viewFor(sync?: () => Promise<void>) {
  const view = new EditorView({
    state: EditorState.create({
      doc: source,
      extensions: [lsp.lspDoc.of(context), ...(sync ? [documentSynced.of(sync)] : [])],
    }),
  });
  views.push(view);
  return view;
}
function format(view: EditorView, workflow: string) {
  return workflow === "document"
    ? formatDocument(view, { explicit: false }).then((result) => result.applied)
    : formatLines(view, 0, source.length);
}
function mutate(view: EditorView, change: string) {
  if (change === "text") view.dispatch({ changes: { from: source.length, insert: " " } });
  else if (change === "selection") view.dispatch({ selection: { anchor: 2 } });
  else if (change === "project") useProject.setState({ project: { root: "/other" } as ProjectInfo });
  else if (change === "settings") useProject.setState({ settings: { editor: { tabSize: 8 } } as Settings });
  else if (change === "opening") useProject.setState({ opening: true, openingPath: "/other" });
  else if (change === "light-edit") useProject.setState({ lightEdit: !useProject.getState().lightEdit });
  else if (change === "indent")
    view.dispatch({ effects: StateEffect.appendConfig.of(formatIndentFacet.of({ tabSize: 8, insertSpaces: false })) });
  else if (change === "state-read-only")
    view.dispatch({ effects: StateEffect.appendConfig.of(EditorState.readOnly.of(true)) });
  else if (change !== "same") {
    const next = {
      ...context,
      ...(change === "path"
        ? { path: "/project/new.php" }
        : change === "document"
          ? { doc: 2 }
          : change === "language"
            ? { language: "html" }
            : change === "read-only"
              ? { readOnly: true }
              : {}),
    };
    view.dispatch({
      effects: StateEffect.reconfigure.of([lsp.lspDoc.of(next), EditorState.readOnly.of(next.readOnly)]),
    });
  }
}
const changes = [
  "same",
  "selection",
  "text",
  "path",
  "document",
  "language",
  "read-only",
  "state-read-only",
  "project",
  "settings",
  "opening",
  "light-edit",
  "indent",
];
const allowed = (change: string) => change === "same" || change === "selection";
beforeEach(() => {
  previous = useProject.getState();
});
afterEach(() => {
  views.splice(0).forEach((view) => view.destroy());
  vi.restoreAllMocks();
  useProject.setState(previous, true);
});

describe.each(["document", "generated-lines"])("%s formatter freshness", (workflow) => {
  it.each(["relocate", "read-only"])("rejects late edits after actual document-model %s", async (change) => {
    const metadata = useDocMeta.getState();
    const off = registerEditorContribution({ id: "format-freshness-test", extensions: (ctx) => lsp.lspDoc.of(ctx) });
    const backend = createMemoryBackend({ [context.path]: source });
    const manager = new DocumentManager(backend, { settings: () => null, notify: vi.fn() }, () => {});
    const host: SlotHost = { onFocus() {}, onBlur() {}, onSelection() {}, onUserEdit() {}, onContextMenu() {} };
    manager.start();
    const model = await manager.acquire(context.path);
    const slot = new EditorSlot(model, "format-freshness", host);
    const view = slot.mount(document.body);
    const reply = deferred<TextEdit[]>();
    const request = vi.spyOn(lsp, "lspRequest").mockImplementation(() => reply.promise);
    const pending = format(view, workflow);
    try {
      await vi.waitFor(() => expect(request).toHaveBeenCalledOnce());
      const text = view.state.doc;
      if (change === "relocate") model.relocate("/project/new.php");
      else model.setReadOnly(true);
      expect(view.state.doc).toBe(text);
      expect(lsp.docOf(view)).toMatchObject(change === "relocate" ? { path: "/project/new.php" } : { readOnly: true });
      reply.resolve(edits);
      expect(await pending).toBe(false);
      expect(model.sync.localText.toString()).toBe(source);
      expect(view.state.doc.toString()).toBe(source);
    } finally {
      reply.resolve([]);
      await Promise.allSettled([pending]);
      slot.dispose();
      manager.resetProject();
      manager.stop();
      off();
      useDocMeta.setState(metadata, true);
    }
  });

  it.each(changes)("refuses stale response after %s while preserving unrelated selection", async (change) => {
    const reply = deferred<TextEdit[]>();
    const request = vi.spyOn(lsp, "lspRequest").mockImplementation(() => reply.promise);
    const view = viewFor();
    const pending = format(view, workflow);
    try {
      await vi.waitFor(() => expect(request).toHaveBeenCalledOnce());
      const text = view.state.doc;
      mutate(view, change);
      if (change !== "text") expect(view.state.doc).toBe(text);
      reply.resolve(edits);
      expect(await pending).toBe(allowed(change));
      expect(view.state.doc.toString()).toBe(allowed(change) ? formatted : source + (change === "text" ? " " : ""));
    } finally {
      reply.resolve([]);
      await Promise.allSettled([pending]);
    }
  });

  it.each(changes)("does not send a stale request after local sync: %s", async (change) => {
    const sync = deferred<void>();
    const waiting = vi.fn(() => sync.promise);
    const request = vi.spyOn(lsp, "lspRequest").mockResolvedValue(edits);
    const view = viewFor(waiting);
    const pending = format(view, workflow);
    try {
      await vi.waitFor(() => expect(waiting).toHaveBeenCalledOnce());
      mutate(view, change);
      sync.resolve();
      expect(await pending).toBe(allowed(change));
      expect(request).toHaveBeenCalledTimes(allowed(change) ? 1 : 0);
    } finally {
      sync.resolve();
      await Promise.allSettled([pending]);
    }
  });

  it.each(["same", "path", "read-only", "project"])(
    "checks the real IPC dispatch guard after global sync: %s",
    async (change) => {
      const sync = deferred<void>();
      const waiting = vi.fn(() => sync.promise);
      const off = registerPendingEdits(waiting);
      vi.spyOn(tauri, "hasInvokeRoute").mockReturnValue(true);
      const command = vi.spyOn(commands, "lspFormat").mockResolvedValue({ status: "ok", data: edits });
      const view = viewFor();
      const pending = format(view, workflow);
      try {
        await vi.waitFor(() => expect(waiting).toHaveBeenCalled());
        mutate(view, change);
        sync.resolve();
        expect(await pending).toBe(change === "same");
        expect(command).toHaveBeenCalledTimes(change === "same" ? 1 : 0);
      } finally {
        sync.resolve();
        await Promise.allSettled([pending]);
        off();
      }
    },
  );
});

it("does not offer a tool install for an error from a retired document", async () => {
  let reject!: (error: Error) => void;
  const reply = new Promise<never>((_resolve, fail) => {
    reject = fail;
  });
  const request = vi.spyOn(lsp, "lspRequest").mockImplementation(() => reply);
  const install = vi.fn();
  const off = registerAction({ id: "tools.promptInstall", title: "Install Missing Tool", run: () => install() });
  const view = viewFor();
  const pending = formatDocument(view, { explicit: false });
  try {
    await vi.waitFor(() => expect(request).toHaveBeenCalledOnce());
    mutate(view, "path");
    reject(
      new IpcError({
        kind: "notFound",
        message: "missing formatter",
        data: { type: "tool", tool: "prettier", missing: true },
      }),
    );
    expect((await pending).applied).toBe(false);
    expect(install).not.toHaveBeenCalled();
    expect(view.state.doc.toString()).toBe(source);
  } finally {
    reject(new Error("finished"));
    await Promise.allSettled([pending]);
    off();
  }
});

it("does not continue formatting later ranges after the document identity changes", async () => {
  const reply = deferred<TextEdit[]>();
  const request = vi.spyOn(lsp, "lspRequest").mockImplementation(() => reply.promise);
  const view = viewFor();
  const pending = formatDocument(view, {
    explicit: false,
    ranges: [
      { from: 0, to: 5 },
      { from: 5, to: source.length },
    ],
  });
  try {
    await vi.waitFor(() => expect(request).toHaveBeenCalledOnce());
    mutate(view, "path");
    reply.resolve(edits);
    expect((await pending).applied).toBe(false);
    expect(request).toHaveBeenCalledOnce();
    expect(view.state.doc.toString()).toBe(source);
  } finally {
    reply.resolve([]);
    await Promise.allSettled([pending]);
  }
});
