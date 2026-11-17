/** Encoded PHP ranges through the workbench with controlled IPC. */
import type { MemoryBackend } from "../src/editor/core/backend";
import { expect, ipcCalls, stubDesktop, test } from "./fixtures";

const CALLER = "/dev/acme-api/app/GuardController.php";
const CONFIG = "/dev/acme-api/config/auth.php";
const SOURCE = `<?php
class GuardController extends BaseController {
public function boot() {$note='😀';$this->middleware("auth:ad\\x6din,web");}
}
`;
const AUTH = "<?php return ['guards'=>['admin'=>[]]];\n";

for (const action of ["definition", "references"] as const) {
  test(`encoded guard ${action} keeps UTF-16 coordinates and opens its target`, async ({ workbench: page }) => {
    await page.evaluate(
      async ({ caller, config, source, auth }) => {
        const backendPath = "/src/editor/core/backend.ts";
        const servicesPath = "/src/workbench/contributions.ts";
        const { docBackend } = await import(/* @vite-ignore */ backendPath);
        const { editorService } = await import(/* @vite-ignore */ servicesPath);
        const backend = docBackend() as MemoryBackend;
        backend.files.set(caller, source);
        backend.files.set(config, auth);
        await editorService().openFile(caller, { asText: true, focus: true });
      },
      { caller: CALLER, config: CONFIG, source: SOURCE, auth: AUTH },
    );
    await expect(page.locator(".cm-content").first()).toContainText("ad\\x6din");
    await stubDesktop(page);
    const column = SOURCE.split("\n")[2]!.indexOf("ad\\x6din");
    await page.evaluate(
      async ({ caller, config, source, auth, column }) => {
        const apiPath = "/src/editor/api.ts";
        const { activeEditor } = await import(/* @vite-ignore */ apiPath);
        const editor = activeEditor()!;
        editor.view.dispatch({ selection: { anchor: source.indexOf("ad\\x6din") + 2 } });
        editor.view.focus();
        const declaration = {
          start: { line: 0, col: auth.indexOf("admin") },
          end: { line: 0, col: auth.indexOf("admin") },
        };
        window.__ipc.handlers.lsp_goto = () => [{ path: config, range: declaration }];
        window.__ipc.handlers.lsp_references = () => ({
          locations: [
            { path: config, range: declaration },
            { path: caller, range: { start: { line: 2, col: column }, end: { line: 2, col: column + 8 } } },
          ],
          incomplete: null,
        });
      },
      { caller: CALLER, config: CONFIG, source: SOURCE, auth: AUTH, column },
    );
    if (action === "definition") {
      await page.keyboard.press("Meta+b");
      await expect
        .poll(() =>
          page.evaluate(async () => {
            const module = "/src/workbench/contributions.ts";
            return (await import(/* @vite-ignore */ module)).editorService().activeCursor();
          }),
        )
        .toMatchObject({ path: CONFIG, line: 0, col: AUTH.indexOf("admin") });
    } else {
      await page.keyboard.press("Alt+F7");
      const usages = page.getByRole("tree", { name: /usages/i });
      await expect(usages.getByRole("treeitem", { name: "Found usages 2 results" })).toBeVisible();
      const row = usages.getByRole("treeitem", { name: /^3 .*auth:/ });
      await expect(row).toContainText("ad\\x6din");
      await row.dblclick();
      await expect
        .poll(() =>
          page.evaluate(async () => {
            const module = "/src/workbench/contributions.ts";
            return (await import(/* @vite-ignore */ module)).editorService().activeCursor();
          }),
        )
        .toMatchObject({ path: CALLER, line: 2, col: column });
    }
    const calls = await ipcCalls(page, action === "definition" ? "lsp_goto" : "lsp_references");
    // Definition lookup clamps a caret at the word boundary back inside `ad`.
    expect(calls[0]).toMatchObject({ pos: { line: 2, col: column + (action === "definition" ? 1 : 2) } });
  });
}
