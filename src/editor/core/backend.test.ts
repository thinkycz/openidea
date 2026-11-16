import { expect, it } from "vitest";
import { createMemoryBackend } from "./backend";

it.each(["php", "phtml", "php3", "php4", "php5", "phps", "phpt"])(
  "identifies browser PHP documents and Save As targets (.%s)",
  async (extension) => {
    const backend = createMemoryBackend({ [`/p/Action.${extension}`]: "<?php class Action {}" });
    const opened = await backend.open(`/p/Action.${extension}`);
    expect(opened.language).toBe("php");
    const untitled = await backend.openUntitled("Untitled-1");
    const saved = await backend.saveAs(untitled.id, `/p/Saved.${extension}`);
    expect(saved.language).toBe("php");
    const languages = await backend.languages();
    expect(languages.find((language) => language.id === opened.language)).toMatchObject({
      name: "PHP",
      extensions: expect.arrayContaining([extension]),
      lineComment: "//",
      blockComment: ["/*", "*/"],
      indent: { tabSize: 4, insertSpaces: true },
    });
  },
);
