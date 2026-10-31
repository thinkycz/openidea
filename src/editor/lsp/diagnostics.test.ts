import { describe, expect, it, vi } from "vitest";
import { EditorState, Text } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { scrollbarMarks } from "@/editor/api";
import { lspDoc } from "./lsp";
import { countsOf, inspectionWidgetExtension } from "./inspectionWidget";
import type { Diagnostic } from "@/bindings";
import {
  diagnosticsAt,
  diagnosticsExtension,
  diagnosticsFor,
  setFileDiagnostics,
  subscribeDiagnostics,
  toLintDiagnostics,
} from "./diagnostics";

const d = (
  line: number,
  col: number,
  endCol: number,
  severity: Diagnostic["severity"] = "error",
  over: Partial<Diagnostic> = {},
): Diagnostic => ({
  range: { start: { line, col }, end: { line, col: endCol } },
  severity,
  message: `m${line}:${col}`,
  source: null,
  code: null,
  ...over,
});

describe("editor diagnostics store", () => {
  it("keeps one list per source and notifies subscribers of the path", () => {
    let calls = 0;
    const off = subscribeDiagnostics("/p/x.rs", () => calls++);
    setFileDiagnostics("/p/x.rs", "rust-analyzer", [d(0, 0, 1)]);
    setFileDiagnostics("/p/x.rs", "cargo", [d(1, 0, 1, "warning")]);
    expect(diagnosticsFor("/p/x.rs").map((x) => x.source)).toEqual(["rust-analyzer", "cargo"]);
    setFileDiagnostics("/p/x.rs", "rust-analyzer", []);
    expect(diagnosticsFor("/p/x.rs").map((x) => x.source)).toEqual(["cargo"]);
    setFileDiagnostics("/p/other.rs", "cargo", [d(0, 0, 1)]);
    expect(calls).toBe(3);
    // Clearing an unknown source is a no-op.
    setFileDiagnostics("/p/x.rs", "unknown", []);
    expect(calls).toBe(3);
    off();
  });
});

describe("toLintDiagnostics", () => {
  const doc = Text.of(["fn main() {", "  let x = 1;", "}"]);

  it("converts ranges, sorts and adds mark classes", () => {
    const out = toLintDiagnostics(doc, [
      d(1, 6, 7, "warning", { unnecessary: true, source: "rustc", code: "unused" }),
      d(0, 3, 7, "error", { deprecated: true }),
    ]);
    expect(out).toEqual([
      { from: 3, to: 7, severity: "error", message: "m0:3", source: undefined, markClass: "cm-lsp-deprecated" },
      {
        from: 18,
        to: 19,
        severity: "warning",
        message: "m1:6",
        source: "rustc unused",
        markClass: "cm-lsp-unnecessary",
      },
    ]);
  });

  it("widens empty ranges to one character", () => {
    expect(toLintDiagnostics(doc, [d(0, 2, 2)])[0]).toMatchObject({ from: 2, to: 3 });
    // At a line end the previous character is used.
    expect(toLintDiagnostics(doc, [d(2, 1, 1)])[0]).toMatchObject({ from: doc.length - 1, to: doc.length });
  });
});

describe("diagnostics when returning to an inactive tab", () => {
  it("clears saved underlines, overview marks and counts after the last producer clears while unmounted", async () => {
    const path = "/p/refresh-on-remount.blade.php";
    const host = document.body.appendChild(document.createElement("div"));
    let view: EditorView | undefined;
    try {
      setFileDiagnostics(path, "phpantom", [d(0, 3, 10, "error", { code: "unknown_variable" })]);
      const state = EditorState.create({
        doc: "{{ $person->userOnly }}",
        extensions: [
          lspDoc.of({ doc: 1, path, language: "php", uiMode: "PHP", readOnly: false }),
          diagnosticsExtension(),
          inspectionWidgetExtension(),
        ],
      });
      view = new EditorView({ state, parent: host });
      await vi.waitFor(() => expect(countsOf(view!).error).toBe(1));
      expect(view.state.facet(scrollbarMarks)).toHaveLength(1);
      expect(diagnosticsAt(view, 4)).toHaveLength(1);
      const saved = view.state;
      view.destroy();
      view = undefined;
      // EditorSlot caches the state, but removes its view/subscription on tab switch.
      setFileDiagnostics(path, "phpantom", []);
      expect(diagnosticsFor(path)).toEqual([]);
      view = new EditorView({ state: saved, parent: host });
      await vi.waitFor(() => expect(countsOf(view!).error).toBe(0));
      expect(diagnosticsAt(view, 4)).toEqual([]);
      expect(view.state.facet(scrollbarMarks)).toEqual([]);
      expect(host.querySelector('[aria-label="No problems found"]')).not.toBeNull();
    } finally {
      view?.destroy();
      host.remove();
      setFileDiagnostics(path, "phpantom", []);
    }
  });
});
