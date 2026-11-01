import { afterEach, describe, expect, it, vi } from "vitest";
import { history } from "@codemirror/commands";
import { ChangeSet, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { reviewedHistory, reviewedTransaction } from "@/editor/core/cm/reviewedHistory";
import { editCommand } from "../appActions";
import * as contributions from "@/workbench/contributions";

describe("Edit menu Undo / Redo", () => {
  let view: EditorView | null = null;
  afterEach(() => {
    view?.destroy();
    view = null;
    vi.restoreAllMocks();
  });

  it("use the focused editor's history, including programmatic edits", () => {
    view = new EditorView({ state: EditorState.create({ doc: "a", extensions: history() }), parent: document.body });
    view.dispatch({ changes: { from: 1, insert: "b" }, userEvent: "input.complete" });
    view.contentDOM.focus();
    editCommand("undo")();
    expect(view.state.doc.toString()).toBe("a");
    editCommand("redo")();
    expect(view.state.doc.toString()).toBe("ab");
  });

  it("keeps reviewed menu undo/redo deferred until the guarded project commit", async () => {
    const restore = vi.fn(async (_group: string, _redo: boolean) => {});
    view = new EditorView({
      doc: "Old",
      extensions: [history(), reviewedHistory(restore, vi.fn())],
      parent: document.body,
    });
    view.dispatch(
      reviewedTransaction(
        view.state,
        ChangeSet.of({ from: 0, to: 3, insert: "New" }, 3),
        { group: "project-edit", action: "apply" },
        true,
      ),
    );
    view.contentDOM.focus();
    editCommand("undo")();
    expect(view.state.doc.toString()).toBe("New");
    expect(restore).toHaveBeenCalledExactlyOnceWith("project-edit", false);
    view.dispatch(
      reviewedTransaction(
        view.state,
        ChangeSet.of({ from: 0, to: 3, insert: "Old" }, 3),
        { group: "project-edit", action: "undo" },
        true,
      ),
    );
    for (let i = 0; i < 5; i++) await Promise.resolve();
    editCommand("redo")();
    expect(view.state.doc.toString()).toBe("Old");
    expect(restore).toHaveBeenLastCalledWith("project-edit", true);
  });

  it("delegates text-field menu history to the browser without touching an editor", () => {
    const descriptor = Object.getOwnPropertyDescriptor(document, "execCommand");
    const exec = vi.fn();
    const input = document.createElement("input");
    document.body.append(input);
    try {
      Object.defineProperty(document, "execCommand", { configurable: true, value: exec });
      input.focus();
      editCommand("undo")();
      editCommand("redo")();
      expect(exec.mock.calls).toEqual([["undo"], ["redo"]]);
    } finally {
      input.remove();
      if (descriptor) Object.defineProperty(document, "execCommand", descriptor);
      else Reflect.deleteProperty(document, "execCommand");
    }
  });

  it("routes custom document history before CodeMirror's independent local history", () => {
    const parent = document.createElement("div");
    parent.dataset.paneId = "custom-pane";
    document.body.appendChild(parent);
    view = new EditorView({ state: EditorState.create({ doc: "local", extensions: history() }), parent });
    view.dispatch({ changes: { from: 5, insert: " edit" }, userEvent: "input" });
    let source = "backend edited";
    vi.spyOn(contributions, "maybeEditorService").mockReturnValue({
      openFile: async () => {},
      openInput: async () => {},
      activePath: () => "/p/notebook.ipynb",
      activeCursor: () => null,
      saveAll: async () => {},
      documentHistory: (command) => {
        source = command === "undo" ? "backend original" : "backend edited";
        return true;
      },
    });
    view.contentDOM.focus();
    editCommand("undo")();
    expect(source).toBe("backend original");
    expect(view.state.doc.toString()).toBe("local edit");
    editCommand("redo")();
    expect(source).toBe("backend edited");
    expect(view.state.doc.toString()).toBe("local edit");
    parent.remove();
  });
});
