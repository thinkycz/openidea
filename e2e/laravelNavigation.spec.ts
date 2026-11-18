/** Workbench actions and editor paths with controlled PHPantom IPC replies. */
import type { Page } from "@playwright/test";
import type { LaravelRouteHandler, LaravelRoutes, LaravelViews } from "../src/bindings";
import type { MemoryBackend } from "../src/editor/core/backend";
import { expect, ipcCalls, stubDesktop, test } from "./fixtures";

const ROOT = "/dev/acme-api";
const SOURCE = `${ROOT}/app/InvoiceController.php`;
const ROUTE = `${ROOT}/routes/web.php`;
const VIEW = `${ROOT}/resources/views/invoices/show.blade.php`;

async function seed(page: Page) {
  await page.evaluate(
    async ({ source, route, view }) => {
      const backendPath = "/src/editor/core/backend.ts";
      const contributionsPath = "/src/workbench/contributions.ts";
      const projectPath = "/src/platform/project.ts";
      const { docBackend } = await import(/* @vite-ignore */ backendPath);
      const { editorService } = await import(/* @vite-ignore */ contributionsPath);
      const { useProject } = await import(/* @vite-ignore */ projectPath);
      const settings = useProject.getState().settings ?? {};
      useProject.setState({ settings: { ...settings, files: { ...settings.files, autoSave: false } } });
      const backend = docBackend() as MemoryBackend;
      backend.files.set(source, "<?php\nview('invoices.show');\nroute('invoice.show');\n");
      backend.files.set(
        route,
        "<?php\n// route declaration\nRoute::view('/invoices/{invoice}', 'invoices.show')->name('invoice.show');\n",
      );
      backend.files.set(view, "<h1>Invoice</h1>\n<p>{{ $invoice->id }}</p>\n");
      await editorService().openFile(source, { asText: true, focus: true });
    },
    { source: SOURCE, route: ROUTE, view: VIEW },
  );
  await expect(page.locator(".cm-content").first()).toContainText("invoice.show");
  await stubDesktop(page);
  await page.evaluate(
    ({ route, view }) => {
      const location = (path: string, line: number) => ({
        path,
        range: { start: { line, col: 0 }, end: { line, col: 4 } },
      });
      window.__ipc.handlers.lsp_laravel_routes = ({ name }) =>
        ({
          routes:
            name === null
              ? [
                  { name: "invoice.show", uri: "invoices/{invoice}", parameters: ["invoice"], fromVendor: false },
                  { name: "package.health", uri: "health", parameters: [], fromVendor: true },
                ]
              : [],
          locations: name === null ? [] : [location(route, 2)],
          incomplete: true,
        }) satisfies LaravelRoutes;
      window.__ipc.handlers.lsp_laravel_views = ({ name }) =>
        ({
          views: name === null ? ["invoices.show", "dashboard"] : [],
          locations: name === null ? [] : [location(view, 1)],
          incomplete: false,
        }) satisfies LaravelViews;
    },
    { route: ROUTE, view: VIEW },
  );
}

async function browse(page: Page, kind: "Routes" | "Views") {
  await page.keyboard.press("Meta+Shift+A");
  const search = page.getByRole("dialog", { name: "Search Everywhere", exact: true });
  await search.getByRole("combobox").fill(`Laravel ${kind}`);
  await expect(search.getByRole("option", { name: new RegExp(`Laravel ${kind}`) })).toBeVisible();
  await search.getByRole("combobox").press("Enter");
  await expect(search).toBeHidden();
  const chooser = page.getByRole("dialog", { name: `Laravel ${kind}`, exact: true });
  await expect(chooser).toBeVisible();
  return chooser;
}

for (const kind of ["Routes", "Views"] as const) {
  test(`Find Action discovers Laravel ${kind} and opens its selected target`, async ({ workbench: page }, info) => {
    await seed(page);
    const chooser = await browse(page, kind);
    await expect(chooser.getByRole("option")).toHaveCount(2);
    if (kind === "Routes") {
      await expect(chooser.getByText(/Results are incomplete/)).toBeVisible();
      await expect(chooser.getByRole("option", { name: /package.health/ })).toContainText("(package)");
    }
    const name = kind === "Routes" ? "invoice.show" : "invoices.show";
    await chooser.getByRole("textbox", { name: "Filter Laravel results" }).fill(name);
    await expect(chooser.getByRole("option")).toHaveCount(1);
    await chooser.getByRole("button", { name: "Show Targets", exact: true }).click();
    await expect(chooser.getByRole("option")).toHaveCount(1);
    await expect(chooser.getByRole("option")).toContainText(
      kind === "Routes" ? "routes/web.php:3:1" : "invoices/show.blade.php:2:1",
    );
    await page.screenshot({ path: info.outputPath("laravel-targets.png"), animations: "disabled" });
    await chooser.getByRole("button", { name: "Open", exact: true }).click();
    await expect(chooser).toBeHidden();
    const target = kind === "Routes" ? ROUTE : VIEW;
    await expect
      .poll(() =>
        page.evaluate(async () => {
          const path = "/src/workbench/contributions.ts";
          return (await import(/* @vite-ignore */ path)).editorService().activeCursor();
        }),
      )
      .toMatchObject({ path: target, line: kind === "Routes" ? 2 : 1, col: 0 });
    const calls = await ipcCalls(page, kind === "Routes" ? "lsp_laravel_routes" : "lsp_laravel_views");
    expect(calls.map((call) => call.name)).toEqual([null, name]);
    expect(calls.every((call) => call.limit === 2000)).toBe(true);
  });
}

test("unsaved editor changes retire displayed route targets and Retry keeps the chosen name", async ({
  workbench: page,
}) => {
  await seed(page);
  const chooser = await browse(page, "Routes");
  await chooser.getByRole("textbox", { name: "Filter Laravel results" }).fill("invoice.show");
  await chooser.getByRole("button", { name: "Show Targets", exact: true }).click();
  await expect(chooser.getByRole("option")).toContainText("routes/web.php:3:1");
  await page.evaluate(async () => {
    const apiPath = "/src/editor/api.ts";
    const { activeEditor } = await import(/* @vite-ignore */ apiPath);
    activeEditor().view.dispatch({ changes: { from: 6, insert: "// unsaved dependency\n" }, userEvent: "input.type" });
  });
  await expect(chooser.getByRole("alert")).toContainText("Project files changed");
  await expect(chooser.getByRole("option")).toHaveCount(0);
  await expect(chooser.getByText(/Results are incomplete/)).toBeVisible();
  await chooser.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(chooser.getByRole("option")).toHaveCount(1);
  expect((await ipcCalls(page, "lsp_laravel_routes")).map((call) => call.name)).toEqual([
    null,
    "invoice.show",
    "invoice.show",
  ]);
  await chooser.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(chooser).toBeHidden();
  await expect(page.locator(".cm-content").first()).toContainText("// unsaved dependency");
  const disk = await page.evaluate(async (source) => {
    const path = "/src/editor/core/backend.ts";
    return ((await import(/* @vite-ignore */ path)).docBackend() as MemoryBackend).files.get(source);
  }, SOURCE);
  expect(disk).not.toContain("unsaved dependency");
});

for (const kind of ["Routes", "Views"] as const) {
  test(`opening a displayed Laravel ${kind} target waits for pending editor edits`, async ({
    workbench: page,
  }, info) => {
    await seed(page);
    const chooser = await browse(page, kind);
    const name = kind === "Routes" ? "invoice.show" : "invoices.show";
    await chooser.getByRole("textbox", { name: "Filter Laravel results" }).fill(name);
    await chooser.getByRole("button", { name: "Show Targets", exact: true }).click();
    await expect(chooser.getByRole("option")).toHaveCount(1);
    await page.evaluate(async () => {
      const backendPath = "/src/editor/core/backend.ts";
      const apiPath = "/src/editor/api.ts";
      const backend = (await import(/* @vite-ignore */ backendPath)).docBackend() as MemoryBackend;
      const original = backend.edit;
      const edit = backend.edit.bind(backend);
      backend.edit = (...args) =>
        new Promise((resolve, reject) => {
          window.__ipc.handlers.fixture_release_target_edit = () => {
            backend.edit = original;
            edit(...args).then(resolve, reject);
          };
        });
      (await import(/* @vite-ignore */ apiPath)).activeEditor()!.view.dispatch({
        changes: { from: 6, insert: "// pending 😀 dependency\n" },
        userEvent: "input.type",
      });
    });
    await chooser.getByRole("button", { name: "Open", exact: true }).click();
    await expect(chooser.getByRole("button", { name: "Open", exact: true })).toBeDisabled();
    await expect(chooser.getByRole("status").last()).toContainText("Loading");
    await expect
      .poll(() => page.evaluate(() => typeof window.__ipc.handlers.fixture_release_target_edit))
      .toBe("function");
    await page.evaluate(() => window.__ipc.handlers.fixture_release_target_edit!({}));
    await expect(chooser.getByRole("alert")).toContainText("Project files changed");
    await expect(chooser.getByRole("option")).toHaveCount(0);
    await expect(page.getByRole("tab", { name: /InvoiceController\.php/ })).toHaveAttribute("aria-selected", "true");
    await page.screenshot({ path: info.outputPath("pending-target-retired.png"), animations: "disabled" });
    await chooser.getByRole("button", { name: "Retry", exact: true }).click();
    await expect(chooser.getByRole("option")).toHaveCount(1);
    expect(
      (await ipcCalls(page, kind === "Routes" ? "lsp_laravel_routes" : "lsp_laravel_views")).map((call) => call.name),
    ).toEqual([null, name, name]);
    const disk = await page.evaluate(async (source) => {
      const path = "/src/editor/core/backend.ts";
      return ((await import(/* @vite-ignore */ path)).docBackend() as MemoryBackend).files.get(source);
    }, SOURCE);
    expect(disk).not.toContain("pending 😀 dependency");
    await chooser.getByRole("button", { name: "Open", exact: true }).click();
    await expect(chooser).toBeHidden();
    await expect
      .poll(() =>
        page.evaluate(async () => {
          const path = "/src/workbench/contributions.ts";
          return (await import(/* @vite-ignore */ path)).editorService().activeCursor()?.path;
        }),
      )
      .toBe(kind === "Routes" ? ROUTE : VIEW);
  });
}

test("Cancel retires an in-flight Views request and ignores its late response", async ({ workbench: page }) => {
  await seed(page);
  await page.evaluate(() => {
    window.__ipc.handlers.lsp_laravel_views = () =>
      new Promise((resolve) => {
        window.__ipc.handlers.fixture_release_reply = () => {
          resolve({ views: ["stale"], locations: [], incomplete: false });
        };
      });
  });
  const chooser = await browse(page, "Views");
  await expect.poll(() => ipcCalls(page, "lsp_laravel_views")).toHaveLength(1);
  await expect(chooser.getByRole("button", { name: "Show Targets" })).toBeDisabled();
  await chooser.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(chooser).toBeHidden();
  const [request] = await ipcCalls(page, "lsp_laravel_views");
  expect(await ipcCalls(page, "cancel_request")).toContainEqual({ id: request!.request });
  await page.evaluate(() => window.__ipc.handlers.fixture_release_reply!({}));
  await expect(chooser).toBeHidden();
});

test("Cancel while edits synchronize prevents any Laravel request", async ({ workbench: page }) => {
  await seed(page);
  await page.evaluate(async () => {
    const path = "/src/editor/api.ts";
    const { registerPendingEdits } = await import(/* @vite-ignore */ path);
    let release!: () => void;
    const barrier = new Promise<void>((resolve) => {
      release = resolve;
    });
    const off = registerPendingEdits(() => barrier);
    window.__ipc.handlers.fixture_release_edits = () => {
      release();
      off();
    };
  });
  const chooser = await browse(page, "Routes");
  expect(await ipcCalls(page, "lsp_laravel_routes")).toHaveLength(0);
  await expect(chooser.getByRole("status").first()).toContainText("Loading");
  await chooser.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.evaluate(() => window.__ipc.handlers.fixture_release_edits!({}));
  await expect(chooser).toBeHidden();
  expect(await ipcCalls(page, "lsp_laravel_routes")).toHaveLength(0);
  expect(await ipcCalls(page, "cancel_request")).toHaveLength(0);
});

const HANDLER_SOURCE = `<?php\nuse App\\Invokable;\nuse Illuminate\\Support\\Facades\\Route;\nRoute::get('/😀', Invokable::class);\n`;
const HANDLER_TARGET = `<?php\nnamespace App;\n/*😀*/ class Invokable { public function __invoke(): string { return 'ok'; } }\n`;
const HANDLER_ROUTE = `${ROOT}/routes/invokable.php`;
const HANDLER_PATH = `${ROOT}/app/Invokable.php`;
const HANDLER_POS = { line: 3, col: HANDLER_SOURCE.split("\n")[3]!.indexOf("Invokable") };
const HANDLER_DECL = { line: 2, col: HANDLER_TARGET.split("\n")[2]!.indexOf("__invoke") };
async function seedHandler(page: Page, incomplete = false) {
  await seed(page);
  await page.evaluate(
    async ({ source, target, sourceText, targetText, pos, decl, incomplete, root }) => {
      const backendPath = "/src/editor/core/backend.ts";
      const contributionPath = "/src/workbench/contributions.ts";
      const backend = (await import(/* @vite-ignore */ backendPath)).docBackend() as MemoryBackend;
      backend.files.set(source, sourceText);
      backend.files.set(target, targetText);
      backend.files.set(
        `${root}/composer.json`,
        JSON.stringify({ require: { "laravel/framework": "^12.0" }, autoload: { "psr-4": { "App\\": "app/" } } }),
      );
      window.__ipc.handlers.lsp_laravel_route_handler = () =>
        ({
          locations: [{ path: target, range: { start: decl, end: { line: decl.line, col: decl.col + 8 } } }],
          incomplete,
        }) satisfies LaravelRouteHandler;
      await (
        await import(/* @vite-ignore */ contributionPath)
      )
        .editorService()
        .openFile(source, { asText: true, focus: true, selection: pos });
    },
    {
      source: HANDLER_ROUTE,
      target: HANDLER_PATH,
      sourceText: HANDLER_SOURCE,
      targetText: HANDLER_TARGET,
      pos: HANDLER_POS,
      decl: HANDLER_DECL,
      incomplete,
      root: ROOT,
    },
  );
  await expect(page.locator(".cm-content").first()).toContainText("Invokable::class");
}
async function invokeHandler(page: Page) {
  await page.keyboard.press("Meta+Shift+A");
  const search = page.getByRole("dialog", { name: "Search Everywhere", exact: true });
  await search.getByRole("combobox").fill("Go to Invokable Route Handler");
  await expect(search.getByRole("option", { name: /Go to Invokable Route Handler/ })).toBeVisible();
  await search.getByRole("combobox").press("Enter");
  await expect(search).toBeHidden();
}
async function expectHandlerTarget(page: Page) {
  await expect(page.getByRole("tab", { name: /Invokable\.php/ })).toHaveAttribute("aria-selected", "true");
  await expect
    .poll(() =>
      page.evaluate(async () => {
        const path = "/src/editor/api.ts";
        const view = (await import(/* @vite-ignore */ path)).activeEditor()!.view;
        const head = view.state.selection.main.head;
        const line = view.state.doc.lineAt(head);
        return { line: line.number - 1, col: head - line.from };
      }),
    )
    .toEqual(HANDLER_DECL);
}
test("Find Action opens one complete invokable implementation at its UTF16 declaration", async ({
  workbench: page,
}) => {
  await seedHandler(page);
  await invokeHandler(page);
  await expectHandlerTarget(page);
  expect((await ipcCalls(page, "lsp_laravel_route_handler")).map((call) => call.pos)).toEqual([HANDLER_POS]);
  await expect(page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true })).toBeHidden();
});
test("incomplete single handlers require explicit opening and keep their warning", async ({
  workbench: page,
}, info) => {
  await seedHandler(page, true);
  await invokeHandler(page);
  const chooser = page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true });
  await expect(chooser).toBeVisible();
  await expect(chooser.getByText(/Results are incomplete/)).toBeVisible();
  await expect(chooser.getByRole("option")).toHaveCount(1);
  await expect(chooser.getByRole("button", { name: "Back", exact: true })).toHaveCount(0);
  await page.screenshot({ path: info.outputPath("handler-incomplete.png"), animations: "disabled" });
  await chooser.getByRole("button", { name: "Open", exact: true }).click();
  await expectHandlerTarget(page);
});
test("an unsaved source edit retires a held handler and Retry uses the current caret", async ({ workbench: page }) => {
  await seedHandler(page);
  await page.evaluate(() => {
    const original = window.__ipc.handlers.lsp_laravel_route_handler!;
    let first = true;
    window.__ipc.handlers.lsp_laravel_route_handler = (args) => {
      if (!first) return original(args);
      first = false;
      return new Promise((resolve) => {
        window.__ipc.handlers.fixture_release_handler = () => resolve(original(args));
      });
    };
  });
  await invokeHandler(page);
  const chooser = page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true });
  await expect.poll(async () => (await ipcCalls(page, "lsp_laravel_route_handler")).length).toBe(1);
  await page.evaluate(async () => {
    const path = "/src/editor/api.ts";
    (await import(/* @vite-ignore */ path))
      .activeEditor()!
      .view.dispatch({ changes: { from: 6, insert: "// unsaved source\n" }, userEvent: "input.type" });
  });
  await expect(chooser.getByRole("alert")).toContainText("Project files changed");
  await page.evaluate(() => window.__ipc.handlers.fixture_release_handler!({}));
  await expect(chooser).toBeVisible();
  await expect(page.getByRole("tab", { name: /invokable\.php/ })).toHaveAttribute("aria-selected", "true");
  await chooser.getByRole("button", { name: "Retry", exact: true }).click();
  await expectHandlerTarget(page);
  expect((await ipcCalls(page, "lsp_laravel_route_handler")).map((call) => call.pos)).toEqual([
    HANDLER_POS,
    { line: HANDLER_POS.line + 1, col: HANDLER_POS.col },
  ]);
  const disk = await page.evaluate(async (path) => {
    const module = "/src/editor/core/backend.ts";
    return ((await import(/* @vite-ignore */ module)).docBackend() as MemoryBackend).files.get(path);
  }, HANDLER_ROUTE);
  expect(disk).toBe(HANDLER_SOURCE);
});
test("Cancel before source synchronization prevents a handler request", async ({ workbench: page }) => {
  await seedHandler(page);
  await page.evaluate(async () => {
    const path = "/src/editor/api.ts";
    let release!: () => void;
    const barrier = new Promise<void>((resolve) => {
      release = resolve;
    });
    const off = (await import(/* @vite-ignore */ path)).registerPendingEdits(() => barrier);
    window.__ipc.handlers.fixture_release_edits = () => {
      release();
      off();
    };
  });
  await invokeHandler(page);
  const chooser = page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true });
  await expect(chooser).toBeVisible();
  expect(await ipcCalls(page, "lsp_laravel_route_handler")).toHaveLength(0);
  await chooser.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.evaluate(() => window.__ipc.handlers.fixture_release_edits!({}));
  await expect(chooser).toBeHidden();
  expect(await ipcCalls(page, "lsp_laravel_route_handler")).toHaveLength(0);
});

test("a held handler reply cannot move focus after the source caret changes", async ({ workbench: page }) => {
  await seedHandler(page);
  await page.evaluate(() => {
    const original = window.__ipc.handlers.lsp_laravel_route_handler!;
    let first = true;
    window.__ipc.handlers.lsp_laravel_route_handler = (args) => {
      if (!first) return original(args);
      first = false;
      return new Promise((resolve) => {
        window.__ipc.handlers.fixture_release_handler = () => resolve(original(args));
      });
    };
  });
  await invokeHandler(page);
  await expect.poll(async () => (await ipcCalls(page, "lsp_laravel_route_handler")).length).toBe(1);
  await page.evaluate(async () => {
    const path = "/src/editor/api.ts";
    const view = (await import(/* @vite-ignore */ path)).activeEditor()!.view;
    view.dispatch({ selection: { anchor: view.state.selection.main.head + 1 }, userEvent: "select" });
    window.__ipc.handlers.fixture_release_handler!({});
  });
  await expect(page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true })).toBeHidden();
  await expect(page.getByRole("tab", { name: /invokable\.php/ })).toHaveAttribute("aria-selected", "true");
  await expect
    .poll(() =>
      page.evaluate(async () => {
        const path = "/src/editor/api.ts";
        const view = (await import(/* @vite-ignore */ path)).activeEditor()!.view;
        const head = view.state.selection.main.head;
        const line = view.state.doc.lineAt(head);
        return { line: line.number - 1, col: head - line.from };
      }),
    )
    .toEqual({ line: HANDLER_POS.line, col: HANDLER_POS.col + 1 });
  await invokeHandler(page);
  await expectHandlerTarget(page);
  expect((await ipcCalls(page, "lsp_laravel_route_handler")).map((call) => call.pos)).toEqual([
    HANDLER_POS,
    { line: HANDLER_POS.line, col: HANDLER_POS.col + 1 },
  ]);
});
test("a displayed handler cannot open from a changed source selection", async ({ workbench: page }) => {
  await seedHandler(page, true);
  await invokeHandler(page);
  await expect(page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true })).toBeVisible();
  await page.evaluate(async () => {
    const apiPath = "/src/editor/api.ts";
    const navigationPath = "/src/features/code-insight/laravel.ts";
    const api = await import(/* @vite-ignore */ apiPath);
    const navigation = await import(/* @vite-ignore */ navigationPath);
    const item = navigation.useLaravelNavigation.getState().items[0];
    const view = api.activeEditor()!.view;
    view.dispatch({ selection: { anchor: view.state.selection.main.head + 1 }, userEvent: "select" });
    await navigation.selectLaravelItem(item);
  });
  await expect(page.getByRole("dialog", { name: "Invokable Route Handlers", exact: true })).toBeHidden();
  await expect(page.getByRole("tab", { name: /invokable\.php/ })).toHaveAttribute("aria-selected", "true");
  const disk = await page.evaluate(async (path) => {
    const module = "/src/editor/core/backend.ts";
    return ((await import(/* @vite-ignore */ module)).docBackend() as MemoryBackend).files.get(path);
  }, HANDLER_ROUTE);
  expect(disk).toBe(HANDLER_SOURCE);
});
