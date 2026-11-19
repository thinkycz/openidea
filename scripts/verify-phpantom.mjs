import { readFileSync } from "node:fs";
import { verifyBundledPhpantom } from "./phpantom-provenance.ts";

const root = new URL("../src-tauri/resources/phpantom/", import.meta.url);
const manifest = JSON.parse(readFileSync(new URL("provenance.json", root), "utf8"));
const source = JSON.parse(readFileSync(new URL("../engine/source.json", import.meta.url), "utf8"));
verifyBundledPhpantom(readFileSync(new URL("phpantom_lsp", root)), manifest, source, (name) =>
  readFileSync(new URL(`../engine/patches/${name}`, import.meta.url)),
);
console.log(`PHPantom ${manifest.version}: checksum and source verified (${manifest.platform})`);
