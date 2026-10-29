import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
const root = new URL("../src-tauri/resources/phpantom/", import.meta.url);
const manifest = JSON.parse(readFileSync(new URL("provenance.json", root), "utf8"));
const digest = createHash("sha256")
  .update(readFileSync(new URL("phpantom_lsp", root)))
  .digest("hex");
if (digest !== manifest.sha256) throw new Error("Bundled PHPantom checksum does not match its provenance");
console.log(`PHPantom ${manifest.version}: checksum verified (${manifest.platform})`);
