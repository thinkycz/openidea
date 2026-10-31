import { describe, it, expect, vi } from "vitest";
import { EditorState, ChangeSet, Transaction } from "@codemirror/state";
import { history, isolateHistory } from "@codemirror/commands";
import { reviewedUndo as undo, reviewedRedo as redo, reviewedHistory, reviewedTransaction } from "./reviewedHistory";

function setup() {
  const restore = vi.fn(async (_group: string, _redo: boolean) => {});
  const error = vi.fn();
  let state = EditorState.create({ doc: "Old unsaved", extensions: [history(), reviewedHistory(restore, error)] });
  const target = {
    get state() {
      return state;
    },
    dispatch(tr: Transaction) {
      state = tr.state;
    },
  };
  const remote = (text: string, action: "apply" | "undo" | "redo") => {
    const changes = ChangeSet.of({ from: 0, to: state.doc.length, insert: text }, state.doc.length);
    state = reviewedTransaction(state, changes, { group: "g1", action }, true).state;
  };
  return {
    restore,
    error,
    target,
    remote,
    get state() {
      return state;
    },
    type(text: string) {
      state = state.update({
        changes: { from: state.doc.length, insert: text },
        annotations: isolateHistory.of("full"),
      }).state;
    },
  };
}

describe("reviewed workspace edit history", () => {
  it("defers local undo until the backend restores every file, then supports redo", async () => {
    const f = setup();
    f.remote("New unsaved", "apply");
    undo(f.target);
    expect(f.state.doc.toString()).toBe("New unsaved");
    await Promise.resolve();
    expect(f.restore).toHaveBeenCalledWith("g1", false);
    f.remote("Old unsaved", "undo");
    expect(f.restore).toHaveBeenCalledTimes(1);
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    redo(f.target);
    expect(f.state.doc.toString()).toBe("Old unsaved");
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    expect(f.restore).toHaveBeenLastCalledWith("g1", true);
    f.remote("New unsaved", "redo");
    expect(f.state.doc.toString()).toBe("New unsaved");
  });
  it("ordinary typing undoes first, without splitting the reviewed edit", async () => {
    const f = setup();
    f.remote("New unsaved", "apply");
    f.type(" later");
    undo(f.target);
    expect(f.state.doc.toString()).toBe("New unsaved");
    expect(f.restore).not.toHaveBeenCalled();
    undo(f.target);
    await Promise.resolve();
    expect(f.restore).toHaveBeenCalledWith("g1", false);
  });
  it("failed guarded undo preserves local history and text for retry", async () => {
    const f = setup();
    f.remote("New unsaved", "apply");
    f.restore.mockRejectedValueOnce(new Error("external change"));
    undo(f.target);
    for (let i = 0; i < 5; i++) await Promise.resolve();
    expect(f.state.doc.toString()).toBe("New unsaved");
    expect(f.error).toHaveBeenCalled();
    undo(f.target);
    await Promise.resolve();
    expect(f.restore).toHaveBeenCalledTimes(2);
  });
});

it("native Edit-menu beforeinput requests shared undo without changing one file locally", async () => {
  const { EditorView } = await import("@codemirror/view");
  const restore = vi.fn(async () => {});
  const view = new EditorView({
    doc: "Old",
    extensions: [history(), reviewedHistory(restore, vi.fn())],
    parent: document.body,
  });
  try {
    view.dispatch(
      reviewedTransaction(
        view.state,
        ChangeSet.of({ from: 0, to: 3, insert: "New" }, 3),
        { group: "native", action: "apply" },
        true,
      ),
    );
    const event = new InputEvent("beforeinput", { inputType: "historyUndo", bubbles: true, cancelable: true });
    view.contentDOM.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(view.state.doc.toString()).toBe("New");
    expect(restore).toHaveBeenCalledExactlyOnceWith("native", false);
  } finally {
    view.destroy();
  }
});
