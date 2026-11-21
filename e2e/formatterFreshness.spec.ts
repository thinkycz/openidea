/** Registered formatting action with held IPC replies and real workbench document moves. */
import type { MemoryBackend } from "../src/editor/core/backend";
import { expect, ipcCalls, stubDesktop, test } from "./fixtures";

const FILE = "/dev/acme-api/app/Formatter.php";
const MOVED = "/dev/acme-api/app/RenamedFormatter.php";
const SOURCE = "<?php /* 😀 */ echo 1;";
const FORMATTED = "<?php /* 😀 */\necho 1;\n";

for (const change of ["same", "rename", "typing", "project"] as const) {
  test(`Reformat Code retires only stale held replies: ${change}`, async ({ workbench: page }) => {
    await page.evaluate(
      async ({ path, text }) => {
        const backendPath = "/src/editor/core/backend.ts";
        const servicesPath = "/src/workbench/contributions.ts";
        const projectPath = "/src/platform/project.ts";
        const { docBackend } = await import(/* @vite-ignore */ backendPath);
        const { editorService } = await import(/* @vite-ignore */ servicesPath);
        const { useProject } = await import(/* @vite-ignore */ projectPath);
        const settings = useProject.getState().settings ?? {};
        useProject.setState({ settings: { ...settings, files: { ...settings.files, autoSave: false } } });
        (docBackend() as MemoryBackend).files.set(path, text);
        await editorService().openFile(path, { asText: true, focus: true });
      },
      { path: FILE, text: SOURCE },
    );
    await expect(page.locator(".cm-content").first()).toContainText("echo 1;");
    await page.keyboard.press("Meta+Shift+A");
    const search = page.getByRole("dialog", { name: "Search Everywhere", exact: true });
    await search.getByRole("combobox").fill("Reformat Code");
    await expect(search.getByRole("option", { name: /Reformat Code/ }).first()).toBeVisible();
    await search.getByRole("combobox").press("Escape");
    await expect(search).toBeHidden();
    await stubDesktop(page);
    await page.evaluate(
      ({ source, formatted }) => {
        window.__ipc.handlers.lsp_format = () =>
          new Promise((resolve) => {
            window.__ipc.handlers.release_formatter_test = () => {
              resolve([
                { range: { start: { line: 0, col: 0 }, end: { line: 0, col: source.length } }, new_text: formatted },
              ]);
              return true;
            };
          });
      },
      { source: SOURCE, formatted: FORMATTED },
    );
    await page.evaluate(async () => {
      const path = "/src/platform/actions.ts";
      const { executeAction } = await import(/* @vite-ignore */ path);
      const editorPath = "/src/editor/api.ts";
      const { activeEditor } = await import(/* @vite-ignore */ editorPath);
      const editor = activeEditor()!;
      window.__ipc.handlers.formatter_test_text = () => editor.view.state.doc.toString();
      let done = false;
      window.__ipc.handlers.formatter_test_done = () => done;
      void executeAction("code.reformat").then(() => {
        done = true;
      });
    });
    await expect.poll(async () => (await ipcCalls(page, "lsp_format")).length).toBe(1);
    await page.evaluate(
      async ({ change, moved }) => {
        const editorPath = "/src/editor/api.ts";
        const backendPath = "/src/editor/core/backend.ts";
        const projectPath = "/src/platform/project.ts";
        const { activeEditor } = await import(/* @vite-ignore */ editorPath);
        const editor = activeEditor()!;
        if (change === "rename") {
          const { docBackend } = await import(/* @vite-ignore */ backendPath);
          (docBackend() as MemoryBackend).move(editor.ctx.doc, moved);
        } else if (change === "typing") {
          editor.view.dispatch({ changes: { from: editor.view.state.doc.length, insert: " // typed" } });
        } else if (change === "project") {
          const { useProject } = await import(/* @vite-ignore */ projectPath);
          const current = useProject.getState().project;
          useProject.setState({ project: { ...current, root: "/different-project" } });
        }
      },
      { change, moved: MOVED },
    );
    await page.evaluate(() => window.__ipc.handlers.release_formatter_test!({}));
    await expect.poll(() => page.evaluate(() => window.__ipc.handlers.formatter_test_done!({}))).toBe(true);
    const text = await page.evaluate(() => window.__ipc.handlers.formatter_test_text!({}));
    expect(text).toBe(change === "same" ? FORMATTED : SOURCE + (change === "typing" ? " // typed" : ""));
    await expect
      .poll(() =>
        page.evaluate(
          async ({ path }) => {
            const module = "/src/editor/core/backend.ts";
            const { docBackend } = await import(/* @vite-ignore */ module);
            return (docBackend() as MemoryBackend).files.get(path);
          },
          { path: change === "rename" ? MOVED : FILE },
        ),
      )
      .toBe(SOURCE);
  });
}
