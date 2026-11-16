import type { CompletionItem } from "../src/bindings";
import type { MemoryBackend } from "../src/editor/core/backend";
import { expect, stubDesktop, test } from "./fixtures";

for (const key of ["Enter", "Tab"]) {
  for (const quote of ["'", '"']) {
    test(`${key} commits a typed middleware guard (${quote}) with undo, redo and save`, async ({ workbench: page }) => {
      const path = "/dev/acme-api/app/GuardController.php";
      const source = `<?php
class GuardController extends BaseController {public function boot() {$label='😀'; $this->middleware(${quote}auth:adOLD,web${quote});}}
`;
      await page.evaluate(
        async ({ path, source }) => {
          const backendPath = "/src/editor/core/backend.ts";
          const servicesPath = "/src/workbench/contributions.ts";
          const projectPath = "/src/platform/project.ts";
          const { docBackend } = await import(/* @vite-ignore */ backendPath);
          const { editorService } = await import(/* @vite-ignore */ servicesPath);
          const { useProject } = await import(/* @vite-ignore */ projectPath);
          const settings = useProject.getState().settings ?? {};
          useProject.setState({ settings: { ...settings, files: { ...settings.files, autoSave: false } } });
          (docBackend() as MemoryBackend).files.set(path, source);
          await editorService().openFile(path, { asText: true, focus: true });
        },
        { path, source },
      );
      await expect(page.locator(".cm-content").first()).toContainText("adOLD");
      await stubDesktop(page);
      await page.evaluate((source) => {
        const start = source.split("\n")[1]!.indexOf("adOLD");
        const item: CompletionItem = {
          label: "admin",
          labelDetail: null,
          labelDescription: "Auth guard",
          detail: "string",
          kind: "property",
          documentation: { kind: "markdown", value: "Auth guard documentation." },
          sortText: "00000",
          filterText: "admin",
          preselect: false,
          deprecated: false,
          insertText: "admin",
          isSnippet: false,
          replaceRange: { start: { line: 1, col: start }, end: { line: 1, col: start + 5 } },
          replaceRangeTab: null,
          additionalEdits: [],
          commitCharacters: [],
          command: null,
          data: null,
          server: 1,
        };
        const other = {
          ...item,
          label: "administrator",
          filterText: "administrator",
          insertText: "administrator",
          sortText: "999",
        };
        window.__ipc.handlers.lsp_completion = () => ({ isIncomplete: false, items: [item, other] });
        window.__ipc.handlers.lsp_resolve_completion = () => item;
      }, source);
      await page
        .locator(".cm-content")
        .first()
        .evaluate(async (content) => {
          const module = "/node_modules/.vite/deps/@codemirror_view.js";
          const { EditorView } = await import(/* @vite-ignore */ module);
          const view = EditorView.findFromDOM(content as HTMLElement)!;
          view.dispatch({ selection: { anchor: view.state.doc.toString().indexOf("adOLD") + 2 } });
          view.focus();
        });
      await page.keyboard.press("Control+Space");
      await expect(page.locator(".cm-tooltip-autocomplete")).toContainText("admin");
      // CodeMirror ignores accept keys during its 75 ms interaction guard.
      await page.waitForTimeout(100);
      await page.keyboard.press(key);
      const text = () =>
        page.evaluate(async () => {
          const module = "/src/editor/api.ts";
          return (await import(/* @vite-ignore */ module)).activeEditor()?.view.state.doc.toString();
        });
      const disk = () =>
        page.evaluate(async (path) => {
          const module = "/src/editor/core/backend.ts";
          return ((await import(/* @vite-ignore */ module)).docBackend() as MemoryBackend).files.get(path);
        }, path);
      const expected = source.replace("adOLD", "admin");
      await expect.poll(text).toBe(expected);
      expect(await disk()).toBe(source);
      await page.keyboard.press("Meta+z");
      await expect.poll(text).toBe(source);
      await page.keyboard.press("Meta+Shift+z");
      await expect.poll(text).toBe(expected);
      await page.keyboard.press("Meta+s");
      await expect.poll(disk).toBe(expected);
    });
  }
}
