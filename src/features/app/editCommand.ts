import { reviewedRedo as redo, reviewedUndo as undo } from "@/editor/core/cm/reviewedHistory";
import { maybeEditorService } from "@/workbench/contributions";
import { EditorView } from "@codemirror/view";

/**
 * History menu items on every OS, and clipboard items on Windows/Linux, act on the
 * focused element like native commands. Editor Undo / Redo respects shared
 * reviewed-edit transactions as well as ordinary history: the native stack misses programmatic edits (templates,
 * completion, refactorings).
 */
export function editCommand(command: "undo" | "redo" | "cut" | "copy" | "paste" | "selectAll") {
  return () => {
    if (command === "undo" || command === "redo") {
      const focused = document.activeElement;
      if (
        focused instanceof Element &&
        focused.closest("[data-editor-area], [data-pane-id]") &&
        maybeEditorService()?.documentHistory?.(command)
      )
        return;
      const view = focused instanceof HTMLElement ? EditorView.findFromDOM(focused) : null;
      if (view) {
        (command === "undo" ? undo : redo)(view);
        return;
      }
    }
    if (command === "paste" && navigator.clipboard?.readText) {
      void navigator.clipboard.readText().then((text) => document.execCommand("insertText", false, text));
      return;
    }
    document.execCommand(command);
  };
}
