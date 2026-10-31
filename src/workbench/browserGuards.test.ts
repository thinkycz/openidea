import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { hideContextMenu, selectableText, showContextMenu, useContextMenuStore } from "@/ui";
import { installBrowserGuards } from "./browserGuards";

let dispose: () => void;

beforeEach(() => {
  dispose = installBrowserGuards({ blockReload: false });
});

afterEach(() => {
  dispose();
  hideContextMenu();
  window.getSelection()?.removeAllRanges();
  document.body.innerHTML = "";
});

function rightClick(el: Element) {
  const e = new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 5, clientY: 6 });
  el.dispatchEvent(e);
  return e;
}

describe("browser guards: context menu", () => {
  it("never shows the native menu, and nothing on chrome", () => {
    document.body.innerHTML = `<button id="b">Run</button>`;
    const e = rightClick(document.getElementById("b")!);
    expect(e.defaultPrevented).toBe(true);
    expect(useContextMenuStore.getState().menu).toBeNull();
  });

  it("shows Copy on selectable read-only text (issue #36)", () => {
    document.body.innerHTML = `<div class="${selectableText}"><p id="p">Hello</p></div>`;
    const range = document.createRange();
    range.selectNodeContents(document.getElementById("p")!);
    window.getSelection()!.addRange(range);
    const e = rightClick(document.getElementById("p")!);
    expect(e.defaultPrevented).toBe(true);
    const menu = useContextMenuStore.getState().menu;
    expect(menu).toMatchObject({ x: 5, y: 6 });
    expect(menu!.items[0]).toMatchObject({ label: "Copy", disabled: false });
  });

  it("keeps a component's own context menu", () => {
    document.body.innerHTML = `<div class="${selectableText}"><p id="p">Hello</p></div>`;
    const p = document.getElementById("p")!;
    p.addEventListener("contextmenu", (e) => showContextMenu(e, [{ label: "Own" }]));
    rightClick(p);
    expect(useContextMenuStore.getState().menu!.items[0]).toMatchObject({ label: "Own" });
  });

  it("keeps out when a handler already prevented the default", () => {
    document.body.innerHTML = `<div class="${selectableText}"><p id="p">Hello</p></div>`;
    const p = document.getElementById("p")!;
    p.addEventListener("contextmenu", (e) => e.preventDefault());
    rightClick(p);
    expect(useContextMenuStore.getState().menu).toBeNull();
  });
});

describe("native macOS field history keys", () => {
  let descriptor: PropertyDescriptor | undefined;
  const exec = vi.fn();
  beforeEach(() => {
    dispose();
    dispose = installBrowserGuards({ blockReload: false, nativeFieldHistory: true });
    descriptor = Object.getOwnPropertyDescriptor(document, "execCommand");
    Object.defineProperty(document, "execCommand", { configurable: true, value: exec });
    exec.mockClear();
  });
  afterEach(() => {
    if (descriptor) Object.defineProperty(document, "execCommand", descriptor);
    else Reflect.deleteProperty(document, "execCommand");
  });
  function field(tag: "input" | "textarea" = "input") {
    const el = document.createElement(tag);
    document.body.append(el);
    el.focus();
    return el;
  }
  function key(el: Element, opts: KeyboardEventInit = {}) {
    const e = new KeyboardEvent("keydown", {
      key: "z",
      code: "KeyZ",
      metaKey: true,
      bubbles: true,
      cancelable: true,
      ...opts,
    });
    el.dispatchEvent(e);
    return e;
  }
  it("restores CmdZ and CmdShiftZ for focused plain inputs and textareas", () => {
    const input = field();
    expect(key(input).defaultPrevented).toBe(true);
    const textarea = field("textarea");
    expect(key(textarea, { shiftKey: true, key: "Z" }).defaultPrevented).toBe(true);
    expect(exec.mock.calls).toEqual([["undo"], ["redo"]]);
  });
  it("uses the logical shortcut key for another keyboard layout", () => {
    const input = field();
    for (const code of ["KeyY", "KeyR", "KeyS"]) expect(key(input, { code }).defaultPrevented).toBe(true);
    expect(exec.mock.calls).toEqual([["undo"], ["undo"], ["undo"]]);
  });
  it("leaves editor-owned, IME and already-handled shortcuts alone", () => {
    const input = field();
    for (const options of [
      { isComposing: true },
      { keyCode: 229 },
      { altKey: true },
      { ctrlKey: true },
      { metaKey: false, ctrlKey: true },
      { key: "x" },
    ])
      expect(key(input, options).defaultPrevented).toBe(false);
    input.addEventListener("keydown", (e) => e.preventDefault(), { once: true });
    key(input);
    const editor = document.createElement("div");
    editor.className = "cm-editor";
    document.body.append(editor);
    editor.append(input);
    input.focus();
    expect(key(input).defaultPrevented).toBe(false);
    expect(exec).not.toHaveBeenCalled();
  });
  it("keeps normal browser history handling when the native workaround is disabled", () => {
    dispose();
    dispose = installBrowserGuards({ blockReload: false, nativeFieldHistory: false });
    const input = field();
    expect(key(input).defaultPrevented).toBe(false);
    expect(exec).not.toHaveBeenCalled();
  });
});
