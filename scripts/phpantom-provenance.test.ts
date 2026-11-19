// @vitest-environment node
import { createHash } from "node:crypto";
import { expect, it } from "vitest";
import { verifyBundledPhpantom } from "./phpantom-provenance.ts";

const sha = (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");
function fixture() {
  const binary = Buffer.from("synthetic executable bytes");
  const patch = Buffer.from("synthetic patch bytes");
  const source = {
    version: "0.11.1-test",
    repository: "https://example.invalid/phpantom",
    commit: "1".repeat(40),
    archive_sha256: "2".repeat(64),
    patches_sha256: sha(patch),
    patches: [{ file: "0001-fixture.patch", sha256: sha(patch) }],
  };
  const provenance = {
    sha256: sha(binary),
    version: source.version,
    upstream_repository: source.repository,
    upstream_commit: source.commit,
    source_archive_sha256: source.archive_sha256,
    patches_sha256: source.patches_sha256,
    patch_count: source.patches.length,
  };
  return { binary, patch, source, provenance, readPatch: () => patch };
}

it("accepts an executable and matching pinned source", () => {
  const f = fixture();
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source, f.readPatch)).not.toThrow();
});

it("retains the bundled executable checksum check", () => {
  const f = fixture();
  expect(() => verifyBundledPhpantom(Buffer.from("changed bytes"), f.provenance, f.source, f.readPatch)).toThrow(
    /checksum/,
  );
});

it("rejects an executable paired with a different active source recipe", () => {
  const f = fixture();
  f.source.version = "0.11.1-other";
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source, f.readPatch)).toThrow(/source recipe/);
});

it.each(["repository", "commit", "archive_sha256", "patches_sha256"] as const)(
  "rejects mismatched source %s",
  (key) => {
    const f = fixture();
    f.source[key] = "mismatched";
    expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source, f.readPatch)).toThrow(/source recipe/);
  },
);

it("rejects identity fields missing from both documents", () => {
  const f = fixture();
  expect(() =>
    verifyBundledPhpantom(
      f.binary,
      { ...f.provenance, upstream_commit: undefined },
      { ...f.source, commit: undefined },
      f.readPatch,
    ),
  ).toThrow(/source recipe commit/);
});

it("rejects inconsistent patch counts", () => {
  const f = fixture();
  f.provenance.patch_count = 2;
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source, f.readPatch)).toThrow(/patch count/);
});

it("rejects malformed patch lists", () => {
  const f = fixture();
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, { ...f.source, patches: null }, f.readPatch)).toThrow(
    /patch count/,
  );
});

it("rejects changed patch contents", () => {
  const f = fixture();
  expect(() =>
    verifyBundledPhpantom(f.binary, f.provenance, f.source, () => Buffer.from("changed patch bytes")),
  ).toThrow(/patch checksum/);
});

it("rejects a misdeclared series even when both identity documents agree", () => {
  const f = fixture();
  f.source.patches_sha256 = f.provenance.patches_sha256 = "3".repeat(64);
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source, f.readPatch)).toThrow(/patch series checksum/);
});

it.each(["../escape.patch", "/escape.patch", "nested/escape.patch", "..\\escape.patch", "file.patch#suffix"])(
  "rejects patch path %s before reading it",
  (file) => {
    const f = fixture();
    f.source.patches[0]!.file = file;
    let reads = 0;
    expect(() =>
      verifyBundledPhpantom(f.binary, f.provenance, f.source, () => {
        reads += 1;
        return f.patch;
      }),
    ).toThrow(/patch filename/);
    expect(reads).toBe(0);
  },
);

it("rejects repeated patch entries", () => {
  const f = fixture();
  f.source.patches.push({ ...f.source.patches[0]! });
  f.provenance.patch_count = 2;
  f.source.patches_sha256 = f.provenance.patches_sha256 = sha(Buffer.concat([f.patch, f.patch]));
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source, f.readPatch)).toThrow(/Duplicate/);
});

it("accepts a source-only upstream build with an empty patch series", () => {
  const f = fixture();
  f.source.patches = [];
  f.provenance.patch_count = 0;
  f.source.patches_sha256 = f.provenance.patches_sha256 = sha(Buffer.alloc(0));
  expect(() => verifyBundledPhpantom(f.binary, f.provenance, f.source)).not.toThrow();
});
