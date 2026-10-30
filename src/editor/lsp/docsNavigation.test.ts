import { afterEach, expect, it, vi } from "vitest";

const line = "    public function validateRequired($attribute, $value)";
const hover = vi.fn();
const openFile = vi.fn(async () => undefined);
vi.mock("@/platform/ipc", async (orig) => ({
  ...(await orig<typeof import("@/platform/ipc")>()),
  isTauri: () => true,
  commands: { lspHover: (...args: unknown[]) => hover(...args) },
}));
vi.mock("@/workbench/contributions", async (orig) => ({
  ...(await orig<typeof import("@/workbench/contributions")>()),
  maybeEditorService: () => ({ openFile }),
}));
vi.mock("./files", () => ({
  readFileLines: async () => [line],
  withDocument: async (_path: string, run: (doc: number) => Promise<unknown>) => run(123),
}));

const { docLinkHandler } = await import("./docs");
const { openPopup, closePopup, useLspPopup } = await import("./popupStore");

function begin() {
  openPopup({
    kind: "doc",
    anchor: { x: 0, y: 0 },
    entries: [{ markup: { kind: "plaintext", value: "rule" }, language: "php" }],
    index: 0,
    themeClasses: "",
  });
}
const documented = (value: string) => ({ status: "ok", data: { contents: { kind: "plaintext", value } } });
afterEach(() => {
  closePopup();
  vi.resetAllMocks();
});

it("opens the rule declaration instead of drilling into a neighboring parameter", async () => {
  begin();
  hover.mockImplementation(async (_id, _doc, pos) =>
    pos.col === line.indexOf("$attribute") ? documented("$attribute = string") : { status: "ok", data: null },
  );
  expect(docLinkHandler("popup")("file:///fixture/ValidatesAttributes.php#L1", new MouseEvent("click"))).toBe(true);
  await vi.waitFor(() =>
    expect(openFile).toHaveBeenCalledWith("/fixture/ValidatesAttributes.php", {
      selection: { line: 0, col: 0 },
      focus: true,
    }),
  );
  expect(hover.mock.calls.map((args) => args[2].col)).not.toContain(line.indexOf("$attribute"));
  expect(useLspPopup.getState().popup).toBeNull();
});

it("retains in-place documentation when the method itself has documentation", async () => {
  begin();
  hover.mockImplementation(async (_id, _doc, pos) =>
    pos.col === line.indexOf("validateRequired") ? documented("method documentation") : { status: "ok", data: null },
  );
  docLinkHandler("popup")("file:///fixture/ValidatesAttributes.php#L1", new MouseEvent("click"));
  await vi.waitFor(() => {
    const popup = useLspPopup.getState().popup;
    expect(popup?.kind === "doc" && popup.entries[popup.index]?.markup.value).toBe("method documentation");
  });
  expect(openFile).not.toHaveBeenCalled();
});

it("honors a link explicitly pointing to a parameter", async () => {
  begin();
  hover.mockResolvedValue(documented("parameter documentation"));
  const col = line.indexOf("$attribute");
  docLinkHandler("popup")(`file:///fixture/ValidatesAttributes.php#L1,${col + 1}`, new MouseEvent("click"));
  await vi.waitFor(() => expect(hover).toHaveBeenCalled());
  expect(hover.mock.calls[0]?.[2]).toEqual({ line: 0, col });
  expect(openFile).not.toHaveBeenCalled();
});
