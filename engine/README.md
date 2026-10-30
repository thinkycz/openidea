# Pinned PHPantom engine

The local OpenStorm candidate bundles PHPantom `0.11.1-openstorm.119` for Apple
Silicon macOS. This is a local rebuild, not an upstream release. `source.json` pins
upstream source, its archive checksum, the unchanged 114-patch PHP Workbench series,
and OpenStorm patches 0115–0119 for JSON/environment resource isolation and precise key destinations. The executable is verified by
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
preserves the pinned engine version through `PHPANTOM_GIT_VERSION`; byte-for-byte
identity across different toolchains is not claimed. A rebuilt engine must pass
OpenStorm client regressions before replacing the resource and updating provenance.
Use the same `PHPANTOM_GIT_VERSION` value for direct Cargo source checks: integration
tests can rebuild the executable in the shared target directory. Treat that directory
as a mutable compiler cache, not a permanent binary receipt. The accepted 115 binary,
licenses and provenance are separately archived at
`artifacts/engines/0.11.1-openstorm.115/`.

The foundation continuation verified archive acquisition, 114 patch applications,
reverse checks immediately after each application, cached-source fingerprints,
deliberate cached-source tamper refusal, and a fresh locked release compilation.
The rebuilt executable reported the pinned version and passed OpenStorm's two
client integration cases and actual-host checked-rename commit case. The original
released executable is retained in the 0.1.25 app and
`artifacts/engines/released-0.11.1-workbench.114/`; its provenance is also preserved in
`engine/releases/0.11.1-workbench.114.json`. Byte identity across builds is not claimed.
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

The 116 candidate prevents PHP examples inside JSON translations and dotenv values from
entering the PHP symbol index. It retains overlay ownership and key-cache invalidation
through the non-PHP resource path, and advertises `phpWorkbenchJsonTranslations` and `phpWorkbenchEnvironmentResources`.
OpenStorm requires that capability before sharing JSON or root dotenv overlays; the
implicit bridge only supplies references, leaving JSON formatting/completion/rename
to the configured JSON provider. The six real-client cases and the actual-host
checked-rename case passed on the rebuilt binary. Engine source and final packaged
acceptance evidence are tracked in `docs/OPENSTORM.md`.

Root dotenv declarations now find PHP/Blade callers through the existing Laravel
string-key index. Environment names/definitions/hover read current open overlays,
including an unsaved root `.env.*`, before disk, and closing an overlay restores disk
semantics. Resource files never contribute PHP declarations. The original 115 JSON
candidate is retained in its signed app/archive and tracked provenance under
`engine/candidates/`; 116 native and full-source evidence is tracked in the progress
record rather than inferred from advertised capabilities.

Patch 0117 reuses the dotenv key-offset parser to navigate past indentation and the
optional `export` prefix. Definition locations use UTF-16 columns, so a subsequent
reference request starts on the declared key. The native 116 destination failure
and actual-client failing regression are retained in the progress/check record.

The Eloquent acceptance extension runs through OpenStorm's actual client with
`OPENSTORM_TEST_ENGINE_LARAVEL_ROOT` pointing at a disposable copy of the pinned
source's `examples/laravel/` corpus installed at exact Laravel 13.37.0. Validate
Composer autoloading and PHP syntax first; install/update dependencies only as an
explicit action with plugins/scripts disabled and an isolated Composer profile.
Do not point this variable at a real user project. The existing
`OPENSTORM_TEST_LARAVEL_ROOT` test uses the separate User/displayName fixture.
`OPENSTORM_TEST_PHPANTOM_BIN` should select the checksum-verified packaged engine.
Run `cargo test -p ide-lsp --test phpantom -- --ignored --nocapture` to exercise all
nine client cases. This creates only test-owned resources and overlays; the
synthetic environment case requires authorization for its fake fixture data.

Patch 0118 keeps an unfinished escaped/raw echo's editing caret before its generated
terminator while retaining reverse source-map anchors. Its public handler checks
completion replacement ranges with ASCII and Unicode prefixes; the OpenStorm client
regression also covers unsaved controller/template data and local shadowing. The
118 binary passes eight client and three actual-host cases. Full-source and native
0.1.30 acceptance remain tracked in the combined progress record.

Patch 0119 retries retained Blade refreshes when startup/configuration publication
changes the input generation during a drain. Its barrier regression must fail
without the retry and pass with it. Source and packaged acceptance remain separate
in `docs/OPENSTORM.md`.

Create a disposable view-data fixture without copying environment files:

```sh
python3 scripts/create-openstorm-view-fixture.py \
  --baseline artifacts/checks/engine-laravel13-37-116/project \
  --php artifacts/toolchains/php-8.4.23-static/php --label view-acceptance
```

The baseline must be the owned, installed pinned example at Laravel 13.37.0.
The script checks source fingerprints, lock/install metadata and PHP syntax before
returning project/profile paths. It never runs Composer or framework commands.
Set `OPENSTORM_TEST_VIEW_ROOT` to its returned project for the ninth real-client
case, covering Blade-first unsaved dependencies through repeated server starts.

Use `--nested-includes` with the fixture generator for a page -> partial chain.
The tenth actual-client scenario verifies unsaved controller/intermediate data,
member navigation, restart and closing an overlay. Its native observations and
separate tab-close protection gap are recorded in `docs/OPENSTORM.md`.
