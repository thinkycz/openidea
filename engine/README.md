# Pinned PHPantom engine

OpenStorm bundles PHPantom `0.11.1-workbench.114` for Apple Silicon macOS.
`source.json` pins upstream source, its archive checksum, and every patch in the
114-patch PHP Workbench series. The released executable is verified by
`node scripts/verify-phpantom.mjs`; its provenance and licenses are under
`src-tauri/resources/phpantom/`.

From the repository root:

```sh
python3 scripts/prepare-phpantom.py
python3 scripts/build-phpantom.py
node scripts/verify-phpantom.mjs
```

Preparation downloads the pinned archive, verifies SHA-256, extracts into the
ignored `artifacts/phpantom/` directory, and applies each verified patch in order.
Environment files are excluded from acquisition and refused as patch targets.
An incomplete acquisition is preserved and reported. It does not touch PHP
Workbench or replace the bundled executable.

The build uses Cargo's locked dependency graph and the pinned upstream
`stubs.lock` (the upstream build script verifies the PHP stub archive).
Use Rust 1.99.1 and record `rustc -Vv`, Cargo version, host architecture and the
printed source/binary SHA-256 receipt when producing a replacement. The recipe
preserves the released engine version through `PHPANTOM_GIT_VERSION`; byte-for-byte
identity across different toolchains is not claimed. A rebuilt engine must pass
OpenStorm client regressions before replacing the resource and updating provenance.

The local continuation verified archive acquisition, 114 patch applications,
reverse checks immediately after each application, cached-source fingerprints,
deliberate cached-source tamper refusal, and a fresh locked release compilation.
The rebuilt executable reported the pinned version and passed OpenStorm's two
client integration cases and actual-host checked-rename commit case. The original
released executable remains bundled; byte identity across builds is not claimed.
Receipts and logs are under `artifacts/checks/phpantom-rebuild*` and
`artifacts/checks/phpantom-rebuilt-*`. The local rust-objcopy/libLLVM warning during
stripping was non-fatal.

Generate a signed app receipt after freezing its source tree:

```sh
python3 scripts/openstorm-build-receipt.py /path/to/OpenStorm.app --profile development
```

Use `--profile release` for an optimized bundle. The receipt verifies bundle
identity, engine checksum and deep/strict signature; it records the dirty source
snapshot instead of implying that uncommitted changes belong to the base commit.
