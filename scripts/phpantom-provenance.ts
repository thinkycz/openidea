import { createHash } from "node:crypto";

const sha256 = (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function verifyBundledPhpantom(
  binary: Uint8Array,
  provenance: unknown,
  source: unknown,
  readPatch?: (name: string) => Uint8Array,
): void {
  if (!record(source) || !record(provenance)) throw new Error("Missing bundled PHPantom source recipe or provenance");
  if (sha256(binary) !== provenance.sha256) throw new Error("Bundled PHPantom checksum does not match its provenance");

  for (const [sourceKey, provenanceKey] of [
    ["version", "version"],
    ["repository", "upstream_repository"],
    ["commit", "upstream_commit"],
    ["archive_sha256", "source_archive_sha256"],
    ["patches_sha256", "patches_sha256"],
  ] as const) {
    if (typeof source[sourceKey] !== "string" || !source[sourceKey] || source[sourceKey] !== provenance[provenanceKey])
      throw new Error(`Bundled PHPantom source recipe ${sourceKey} does not match its provenance`);
  }
  if (!Array.isArray(source.patches) || source.patches.length !== provenance.patch_count)
    throw new Error("Bundled PHPantom source recipe patch count does not match its provenance");

  const series = createHash("sha256");
  const names = new Set();
  for (const patch of source.patches) {
    if (!record(patch)) throw new Error("Invalid PHPantom source patch filename");
    const name = patch.file;
    if (typeof name !== "string" || !/^[a-zA-Z0-9][a-zA-Z0-9._-]*\.patch$/.test(name))
      throw new Error("Invalid PHPantom source patch filename");
    if (names.has(name)) throw new Error("Duplicate PHPantom source patch filename");
    names.add(name);
    if (!readPatch) throw new Error("Missing PHPantom source patch reader");
    const bytes = readPatch(name);
    if (sha256(bytes) !== patch.sha256) throw new Error(`PHPantom source patch checksum mismatch: ${name}`);
    series.update(bytes);
  }
  if (series.digest("hex") !== source.patches_sha256)
    throw new Error("PHPantom source patch series checksum does not match its source recipe");
}
