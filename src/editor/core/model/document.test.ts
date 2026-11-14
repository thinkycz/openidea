import { afterEach, describe, expect, it, vi } from "vitest";
import { ask } from "@/workbench/messages";
vi.mock("@/workbench/messages", () => ({ ask: vi.fn() }));
import { EditorState } from "@codemirror/state";
import { language as languageFacet } from "@codemirror/language";
import { remoteChange } from "@/editor/api";
import { createMemoryBackend, type MemoryBackend } from "../backend";
import { _resetLanguages } from "../cm/languages";
import { DocumentManager, ModifiedRanges, saveCleanupChanges, useDocMeta } from "./document";
import { EditorSlot, type SlotHost } from "./slot";

const host: SlotHost = {
  onFocus: () => {},
  onBlur: () => {},
  onSelection: () => {},
  onUserEdit: () => {},
  onContextMenu: () => {},
};

async function settle() {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
}

function setup(files: Record<string, string>, settings = {}) {
  _resetLanguages();
  const backend: MemoryBackend = createMemoryBackend(files);
  const scheduled: Array<() => void> = [];
  const manager = new DocumentManager(backend, { settings: () => settings, notify: vi.fn() }, (fn) =>
    scheduled.push(fn),
  );
  manager.start();
  const frame = async () => {
    scheduled.splice(0).forEach((f) => f());
    await settle();
  };
  return { backend, manager, frame };
}

afterEach(() => {
  useDocMeta.setState({}, true);
  vi.mocked(ask).mockReset();
});

describe("DocumentManager + slots", () => {
  it("shares one document between split slots and syncs edits to the backend", async () => {
    const { backend, manager, frame } = setup({ "/p/a.txt": "hello\n" });
    const model = await manager.acquire("/p/a.txt");
    expect(await manager.acquire("/p/a.txt")).toBe(model);
    const left = new EditorSlot(model, "t1", host);
    const right = new EditorSlot(model, "t2", host, { from: left });
    left.dispatchLocal({ changes: { from: 5, insert: " world" } });
    expect(right.currentState().doc.toString()).toBe("hello world\n");
    expect(useDocMeta.getState()["/p/a.txt"]?.dirty).toBe(true);
    await frame();
    expect(backend.docs.get(model.doc)?.text).toBe("hello world\n");
    // Undo history stays per slot: the mirrored change is not undoable on the right.
    right.dispatchLocal({ changes: { from: 0, insert: ">" } });
    await frame();
    expect(left.currentState().doc.toString()).toBe(">hello world\n");
    expect(backend.docs.get(model.doc)?.text).toBe(">hello world\n");
  });

  it("applies backend patches to every slot, annotated as remote", async () => {
    const { backend, manager, frame } = setup({ "/p/a.txt": "abc" });
    const model = await manager.acquire("/p/a.txt");
    const a = new EditorSlot(model, "t1", host);
    const b = new EditorSlot(model, "t2", host);
    let sawRemote = false;
    const spy = vi.spyOn(a, "applyExternal").mockImplementation(function (this: EditorSlot, changes, annotations) {
      sawRemote = annotations.some((x) => x.type === remoteChange);
      this.state = this.state.update({ changes, annotations }).state;
    });
    backend.patch(model.doc, [{ from: 3, to: 3, insert: "def" }]);
    expect(sawRemote).toBe(true);
    expect(a.currentState().doc.toString()).toBe("abcdef");
    expect(b.currentState().doc.toString()).toBe("abcdef");
    spy.mockRestore();
    // A local edit after the patch uses the new base version.
    b.dispatchLocal({ changes: { from: 0, insert: "0" } });
    await frame();
    expect(backend.docs.get(model.doc)?.text).toBe("0abcdef");
  });

  it("trims modified lines and adds the final newline on save", async () => {
    const { backend, manager, frame } = setup({ "/p/a.txt": "keep   \nline" });
    const model = await manager.acquire("/p/a.txt");
    const slot = new EditorSlot(model, "t1", host);
    slot.dispatchLocal({ changes: { from: 12, insert: " x   " } });
    const save = model.save("explicit");
    await frame();
    await save;
    expect(backend.files.get("/p/a.txt")).toBe("keep   \nline x\n");
    expect(useDocMeta.getState()["/p/a.txt"]?.dirty).toBe(false);
  });

  it("closes the backend document when the last slot goes away", async () => {
    const { backend, manager } = setup({ "/p/a.txt": "x" });
    const model = await manager.acquire("/p/a.txt");
    const a = new EditorSlot(model, "t1", host);
    const b = new EditorSlot(model, "t2", host);
    a.dispose();
    await settle();
    expect(backend.docs.size).toBe(1);
    b.dispose();
    await settle();
    expect(backend.docs.size).toBe(0);
    expect(manager.get("/p/a.txt")).toBeUndefined();
  });

  it("loads the document's own mode over its language's (Angular templates)", async () => {
    const { manager } = setup({ "/p/angular.json": "{}", "/p/src/app.html": '<p [title]="name()"></p>' });
    const model = await manager.acquire("/p/src/app.html");
    expect(model.getMeta().uiMode).toBe("Angular Template");
    await model.loadLanguage();
    const slot = new EditorSlot(model, "t1", host);
    expect(slot.currentState().facet(languageFacet)?.name).toBe("angular");
  });

  it("takes the document's mode from a language change", async () => {
    const { manager } = setup({ "/p/app.txt": "<p></p>" });
    const model = await manager.acquire("/p/app.txt");
    model.setLanguage("html", "Angular Template");
    expect(model.getMeta().uiMode).toBe("Angular Template");
  });

  it("follows a change of the document's mode alone", async () => {
    const { manager } = setup({ "/p/app.html": "<p></p>" });
    const model = await manager.acquire("/p/app.html");
    const slot = new EditorSlot(model, "t1", host);
    model.setLanguage("html", "Angular Template");
    await model.loadLanguage();
    expect(model.getMeta().uiMode).toBe("Angular Template");
    expect(slot.currentState().facet(languageFacet)?.name).toBe("angular");
    model.setLanguage("html", null);
    expect(model.getMeta().uiMode).not.toBe("Angular Template");
    expect(slot.currentState().facet(languageFacet)?.name).not.toBe("angular");
  });

  it("Save All rejects a failed document save after saving the remaining dirty documents", async () => {
    const { backend, manager } = setup({ "/p/a.txt": "a\n", "/p/b.txt": "b\n" }, { files: { autoSave: false } });
    try {
      const a = await manager.acquire("/p/a.txt");
      const b = await manager.acquire("/p/b.txt");
      backend.patch(a.doc, [{ from: 0, to: 0, insert: "changed " }]);
      backend.patch(b.doc, [{ from: 0, to: 0, insert: "changed " }]);
      const save = backend.save.bind(backend);
      vi.spyOn(backend, "save").mockImplementation(async (id) => {
        if (id === a.doc) throw new Error("disk full");
        return await save(id);
      });
      await expect(manager.saveAll("explicit")).rejects.toThrow("disk full");
      expect(backend.files.get("/p/a.txt")).toBe("a\n");
      expect(backend.files.get("/p/b.txt")).toBe("changed b\n");
      expect(a.getMeta().dirty).toBe(true);
      expect(b.getMeta().dirty).toBe(false);
    } finally {
      manager.stop();
    }
  });
});

describe("save cleanup", () => {
  it("keeps the caret line and untouched lines", () => {
    const state = EditorState.create({ doc: "a  \nb  \nc  " });
    const m = new ModifiedRanges();
    m.ranges = [
      [0, 3],
      [8, 11],
    ];
    const changes = saveCleanupChanges(state, m, { trim: true, finalNewline: true, protectLines: new Set([3]) });
    expect(changes).toEqual([
      { from: 1, to: 3 },
      { from: 11, to: 11, insert: "\n" },
    ]);
  });
});

describe("external save conflicts", () => {
  async function conflict() {
    const state = setup({ "/p/a.txt": "original\n" });
    const model = await state.manager.acquire("/p/a.txt");
    const slot = new EditorSlot(model, "t1", host);
    slot.dispatchLocal({ changes: { from: 0, to: 8, insert: "local" } });
    await state.frame();
    state.backend.files.set("/p/a.txt", "external\n");
    return { ...state, model, slot };
  }

  it("pauses autosave without changing disk or marking the buffer clean", async () => {
    const { model, backend } = await conflict();
    await model.save("auto");
    expect(ask).not.toHaveBeenCalled();
    expect(backend.files.get("/p/a.txt")).toBe("external\n");
    expect(model.getMeta().dirty).toBe(true);
  });

  it("keeps both versions when overwrite is cancelled", async () => {
    const { model, backend } = await conflict();
    vi.mocked(ask).mockResolvedValue({ button: "cancel", checked: false });
    await expect(model.save("explicit")).rejects.toMatchObject({ kind: "cancelled" });
    expect(backend.files.get("/p/a.txt")).toBe("external\n");
    expect(model.getMeta().dirty).toBe(true);
    expect(vi.mocked(ask).mock.calls[0]![0].buttons).toContainEqual({ id: "cancel", label: "Cancel", primary: true });
  });

  it("writes the editor version only after explicit confirmation", async () => {
    const { model, backend } = await conflict();
    vi.mocked(ask).mockResolvedValue({ button: "overwrite", checked: false });
    await model.save("explicit");
    expect(ask).toHaveBeenCalledOnce();
    expect(backend.files.get("/p/a.txt")).toBe("local\n");
    expect(model.getMeta().dirty).toBe(false);
  });

  it("asks again if the disk changes while confirmation is open", async () => {
    const { model, backend } = await conflict();
    vi.mocked(ask)
      .mockImplementationOnce(async () => {
        backend.files.set("/p/a.txt", "new disk\n");
        return { button: "overwrite", checked: false };
      })
      .mockResolvedValueOnce({ button: "cancel", checked: false });
    await expect(model.save("explicit")).rejects.toMatchObject({ kind: "cancelled" });
    expect(ask).toHaveBeenCalledTimes(2);
    expect(backend.files.get("/p/a.txt")).toBe("new disk\n");
    expect(model.getMeta().dirty).toBe(true);
  });

  it("retains a conflicted buffer when its last tab closes", async () => {
    const { model, slot, manager, backend } = await conflict();
    slot.dispose();
    await settle();
    expect(manager.get("/p/a.txt")).toBe(model);
    expect(model.closed).toBe(false);
    const reopened = new EditorSlot(await manager.acquire("/p/a.txt"), "reopened", host);
    expect(reopened.currentState().doc.toString()).toBe("local\n");
    expect(backend.files.get("/p/a.txt")).toBe("external\n");
  });

  it("propagates a cancelled Save All to the quit guard", async () => {
    const { manager } = await conflict();
    vi.mocked(ask).mockResolvedValue({ button: "cancel", checked: false });
    await expect(manager.saveAll("explicit")).rejects.toMatchObject({ kind: "cancelled" });
  });
});

describe("reviewed refactoring history across document slots", () => {
  it("undo in one target requests one shared restore and consumes both histories", async () => {
    const { manager, backend } = setup({ "/p/a.txt": "Old", "/p/b.txt": "Old" });
    const a = await manager.acquire("/p/a.txt");
    const b = await manager.acquire("/p/b.txt");
    const left = new EditorSlot(a, "a", host);
    const split = new EditorSlot(a, "a-split", host);
    const other = new EditorSlot(b, "b", host);
    backend.restoreReviewed = vi.fn(async (_group, redo) => {
      for (const model of [a, b])
        backend.patch(model.doc, [{ from: 0, to: 3, insert: redo ? "New" : "Old" }], {
          group: "g",
          action: redo ? "redo" : "undo",
        });
    });
    for (const model of [a, b])
      backend.patch(model.doc, [{ from: 0, to: 3, insert: "New" }], { group: "g", action: "apply" });
    const { reviewedUndo, reviewedRedo } = await import("../cm/reviewedHistory");
    const target = (slot: EditorSlot) => ({
      get state() {
        return slot.currentState();
      },
      dispatch(tr: import("@codemirror/state").Transaction) {
        slot.state = tr.state;
      },
    });
    reviewedUndo(target(left));
    await settle();
    expect(backend.restoreReviewed).toHaveBeenCalledExactlyOnceWith("g", false);
    for (const slot of [left, split, other]) expect(slot.currentState().doc.toString()).toBe("Old");
    reviewedRedo(target(split));
    await settle();
    expect(backend.restoreReviewed).toHaveBeenLastCalledWith("g", true);
    for (const slot of [left, split, other]) expect(slot.currentState().doc.toString()).toBe("New");
    manager.stop();
  });
});

describe("reviewed move-only history", () => {
  it("keeps one document and shared undo tokens in both splits when no text changes", async () => {
    const { manager, backend } = setup({ "/p/Invoice.php": "<?php\n" });
    const model = await manager.acquire("/p/Invoice.php");
    const left = new EditorSlot(model, "left", host);
    const right = new EditorSlot(model, "right", host);
    left.dispatchLocal({ selection: { anchor: 4 } });
    backend.restoreReviewed = vi.fn(async (_group, redo) => {
      backend.move(model.doc, redo ? "/p/Receipt.php" : "/p/Invoice.php");
      backend.patch(model.doc, [], { group: "move", action: redo ? "redo" : "undo" });
    });
    backend.move(model.doc, "/p/Receipt.php");
    backend.patch(model.doc, [], { group: "move", action: "apply" });
    const { reviewedUndo, reviewedRedo } = await import("../cm/reviewedHistory");
    const target = (slot: EditorSlot) => ({
      get state() {
        return slot.currentState();
      },
      dispatch(tr: import("@codemirror/state").Transaction) {
        slot.state = tr.state;
      },
    });
    expect(reviewedUndo(target(right))).toBe(true);
    await settle();
    expect(backend.restoreReviewed).toHaveBeenCalledExactlyOnceWith("move", false);
    expect(model.path).toBe("/p/Invoice.php");
    expect(left.currentState().selection.main.head).toBe(4);
    expect(reviewedRedo(target(left))).toBe(true);
    await settle();
    expect(backend.restoreReviewed).toHaveBeenLastCalledWith("move", true);
    expect(model.path).toBe("/p/Receipt.php");
    for (const slot of [left, right]) expect(slot.currentState().doc.toString()).toBe("<?php\n");
    expect(manager.byId.size).toBe(1);
    manager.stop();
  });
});

describe("reviewed move during document acquisition", () => {
  it("retargets an in-flight open and shares it with an acquire of the new path", async () => {
    const { manager, backend } = setup({ "/p/Invoice.php": "<?php\n" });
    let release!: () => void;
    let opened!: () => void;
    const barrier = new Promise<void>((resolve) => {
      release = resolve;
    });
    const ready = new Promise<void>((resolve) => {
      opened = resolve;
    });
    const open = backend.open.bind(backend);
    backend.open = async (path) => {
      const info = await open(path);
      opened();
      await barrier;
      return info;
    };
    const pending = manager.acquire("/p/Invoice.php");
    await ready;
    const doc = [...backend.docs.keys()][0]!;
    backend.move(doc, "/p/Receipt.php");
    backend.patch(doc, [{ from: 6, to: 6, insert: "// Renamed while opening.\n" }]);
    const retargeted = manager.acquire("/p/Receipt.php");
    expect(retargeted).toBe(pending);
    release();
    const model = await pending;
    expect(model.path).toBe("/p/Receipt.php");
    expect(manager.get("/p/Receipt.php")).toBe(model);
    expect(manager.get("/p/Invoice.php")).toBeUndefined();
    expect(backend.docs.get(doc)?.refs).toBe(1);
    expect(model.sync.localText.toString()).toBe("<?php\n// Renamed while opening.\n");
    manager.stop();
  });
});
