/**
 * Makes the webview behave like a desktop app: never the browser context menu
 * (its Reload/AutoFill items would act on the webview), no reload/print/
 * save-page/history shortcuts, no navigation on file drop. Copy/paste/undo
 * shortcuts stay untouched in editors. Native macOS fields use browser edit
 * commands for history when menu history routes through the IDE. A right-click no component handled on selected or
 * selectable read-only text gets IDEA's Copy menu instead of nothing.
 */
import { isImeKey } from "@/platform/ime";
import { isTauri } from "@/platform/ipc";
import { guessOs } from "@/platform/platformInfo";
import { copyMenuFor, showContextMenu, useContextMenuStore } from "@/ui";

const BLOCKED = new Set([
  "Meta+P",
  "Ctrl+P",
  "Meta+S",
  "Meta+Shift+S",
  "Meta+O",
  "Meta+G",
  "Meta+Shift+G",
  "Meta+[",
  "Meta+]",
  "Meta+Alt+I",
  "Meta+U",
]);
const RELOAD = new Set(["Meta+R", "Meta+Shift+R", "Ctrl+R", "Ctrl+Shift+R", "F5", "Shift+F5"]);

function keyId(e: KeyboardEvent) {
  const key = e.code.startsWith("Key")
    ? e.code.slice(3)
    : e.code === "BracketLeft"
      ? "["
      : e.code === "BracketRight"
        ? "]"
        : e.key;
  return [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Meta", key]
    .filter(Boolean)
    .join("+");
}

export function installBrowserGuards(opts: { blockReload?: boolean; nativeFieldHistory?: boolean } = {}): () => void {
  const blockReload = opts.blockReload ?? import.meta.env.PROD;
  const nativeFieldHistory = opts.nativeFieldHistory ?? (isTauri() && guessOs() === "macos");
  let menuBefore: unknown = null;
  const beforeContextMenu = () => {
    menuBefore = useContextMenuStore.getState().menu;
  };
  const onContextMenu = (e: MouseEvent) => {
    const handled = e.defaultPrevented || useContextMenuStore.getState().menu !== menuBefore;
    e.preventDefault();
    if (handled) return;
    const items = copyMenuFor(e.target);
    if (items) showContextMenu(e, items);
  };
  const onKeyDown = (e: KeyboardEvent) => {
    const id = keyId(e);
    // AppKit no longer supplies a native Undo role. Keep its field shortcuts
    // without taking history away from editors, IME or component/keymap handlers.
    if (
      nativeFieldHistory &&
      !e.defaultPrevented &&
      !isImeKey(e) &&
      e.metaKey &&
      !e.ctrlKey &&
      !e.altKey &&
      e.key.toLowerCase() === "z"
    ) {
      const active = document.activeElement;
      if (
        active instanceof HTMLElement &&
        !active.closest(".cm-editor") &&
        (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement || active.isContentEditable)
      ) {
        e.preventDefault();
        document.execCommand(e.shiftKey ? "redo" : "undo");
        return;
      }
    }
    if (BLOCKED.has(id) || (blockReload && RELOAD.has(id))) e.preventDefault();
  };
  const onDrag = (e: DragEvent) => {
    if (!(e.target as Element | null)?.closest?.("[data-drop-target]")) e.preventDefault();
  };
  window.addEventListener("contextmenu", beforeContextMenu, true);
  window.addEventListener("contextmenu", onContextMenu);
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("dragover", onDrag);
  window.addEventListener("drop", onDrag);
  return () => {
    window.removeEventListener("contextmenu", beforeContextMenu, true);
    window.removeEventListener("contextmenu", onContextMenu);
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("dragover", onDrag);
    window.removeEventListener("drop", onDrag);
  };
}
