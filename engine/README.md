# Pinned PHPantom engine

The local OpenStorm development candidate uses PHPantom `0.11.1-openstorm.143`
for Apple Silicon macOS. This is a local rebuild, not an upstream release.
Source131 adds migration-derived declaration origins;132 follows registration
order for container bindings and aliases within a provider group. Source133
retires derived facade members and completion candidates after binding changes,
including publication by older calculations. Source134 preserves retriable stale
errors for definition/type-definition and implementation while a semantic
publication is pending. Source135 adds positional/named Route::match controller
action intelligence. Source136 selects route URI arguments by their PHP binding
identity, retaining unknown metadata for ambiguous arguments. Source137 follows
known registrar chains inside controller groups. Source138 resolves runtime
class-string controller groups, preserves enclosing owners for leaf controller
attributes, and refuses unknown nested bindings rather than borrowing an outer
owner. Runtime class strings preserve absolute lookup through completion.
Source139 completes controller actions inside arrow-function group callbacks,
including nested ownership and unknown-binding barriers.
Source140 follows array callback registrations and parenthesized route expressions.
Source141 shares static/fluent controller-group decoding for attributes, named
parameters and callback lists, with unknown-binding barriers.
Source142 shares inherited/rooted group namespace context, runtime controller
qualification and deferred-callable barriers.
Source143 adds explicit Controller@method action references, method completion
and namespace-aware qualification while preserving the class string. Full pinned source
tests pass18,457 checks plus the separate FIFO child (17 ignored).
Examples, Clippy, formatting, source fingerprints and the optimized build pass.
Current OpenStorm client/host and packaged native acceptance are tracked in
`docs/OPENSTORM.md`; source checks alone do not establish native acceptance.
Earlier component, recovery and shortcut evidence remains there. The physical
user-controlled Dock focus case remains unverified, and no full-area or release
acceptance is claimed. The unaccepted129 source and its failed suite result
remain reproducible.

`source.json` pins the upstream source, archive checksum, unchanged114-patch PHP
Workbench series and OpenStorm patches0115–0143. The bundled executable is
verified by `node scripts/verify-phpantom.mjs`; its provenance and licenses are
under `src-tauri/resources/phpantom/`. Source checks and client/host checks do not
replace packaged native acceptance; broader component and view-data work remains
open in `docs/OPENSTORM.md`.

From the repository root:

```sh
python3 scripts/prepare-phpantom.py
python3 scripts/build-phpantom.py
node scripts/verify-phpantom.mjs
```

For the full source regression suite on macOS, use the canonical temporary root.
The backend returns canonical file URIs; two inherited autoload tests compare
those URIs with paths constructed directly from the temporary root. A `/var`
symlink root otherwise differs from the equivalent `/private/var` target. Keep
this setup explicit rather than changing production navigation to return aliases.

```sh
OPENSTORM_ENGINE_SOURCE="$(python3 scripts/prepare-phpantom.py)"
OPENSTORM_TEST_TMPDIR="$(python3 -c 'import os,tempfile; print(os.path.realpath(tempfile.gettempdir()))')"
OPENSTORM_ENGINE_VERSION="$(python3 -c 'import json; print(json.load(open("engine/source.json"))["version"])')"
TMPDIR="$OPENSTORM_TEST_TMPDIR" PHPANTOM_GIT_VERSION="$OPENSTORM_ENGINE_VERSION" cargo test --manifest-path "$OPENSTORM_ENGINE_SOURCE/Cargo.toml" --release --locked -- --test-threads=2
```

Both recipe scripts accept `--pin /path/to/source-manifest.json`. This allows a
staged source candidate to be reconstructed, tested and built while
`engine/source.json` and the bundled engine still identify the accepted version.
For example, the archived130 source uses
`engine/candidates/0.11.1-openstorm.130-source.json`. The earlier129 manifest
remains reproducible with its failed full-suite result recorded. A source manifest contains
the upstream archive and complete checksummed patch series; it is distinct from
a binary candidate's provenance JSON. An explicit pin uses the same integrity
checks and cache fingerprint as the default. Neither build command replaces the
bundled executable. Keep source validation separate from app acceptance: a new bundle remains a
development candidate until its native delivery gates pass.

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
14 client cases. This creates only test-owned resources and overlays; the
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

Patch 0120 adds request-key hover using the same current rule resolution as
navigation. It shows literal rules, labels unevaluated expressions, preserves
incomplete-rule information and links inherited keys to their declaring request.
The eleventh actual-client case covers policies, field documentation/navigation,
unsaved inherited rule changes, server restart and retired field documentation.
Use the pinned 120 Laravel 13.37 corpus for `OPENSTORM_TEST_ENGINE_LARAVEL_ROOT`;
its source fingerprints include the new documentation comment. Native/source
acceptance remains tracked in the combined progress record.

Patch 0121 preserves ContentModified for hover when an admitted source update
has not finished. The client retries this explicit error within one request
deadline, cancelling on source edits, document replacement or engine restart.
Genuine missing documentation remains final. The twelfth actual-client scenario
requests documentation immediately after opening the pinned Demo and after each
restart, without completion/navigation warmup. The 120 Laravel corpus remains
compatible because 0121 changes no example source. Native acceptance is tracked
in `docs/OPENSTORM.md`; a candidate pin does not imply full release acceptance.

The 122 candidate completes literal Laravel validation rule names from the installed
framework and opens their documentation/declarations. Name-only edits preserve
parameters and imports, with UTF-16 ranges and incomplete-result status retained.
EOF completion uses a private recovered parse; ambiguous receiver alternatives
are refused. Eight new public engine tests, all 13 OpenStorm client cases and all
3 host cases pass against the final archived normal-release executable. Dynamic
rule extensions and computed rule arrays remain outside this catalog slice.
Native 0.1.36/0.1.37 candidates remain unaccepted: the client link fallback is
corrected, but a post-restart resolution stall is under investigation. Evidence
and acceptance gates remain in docs/OPENSTORM.md.

Patch 0123 detects circular pending ownership across resolver threads before a
wait, using the existing independent-resolution and per-thread cycle-break paths.
Normal contention continues to share the cached result; wait edges are released
on completion and unwind. Two- and three-owner concurrent mixin regressions retain
all methods without hitting the escape hatch. The final normal-release binary
passes 18,379 source tests (17 ignored), all 14 actual-client and all 4 actual-host
cases, including background reference widening through three server starts with
framework source open. The Laravel analyzer retains exactly its three intentional
errors. The signed 0.1.38 native rule-name/restart slice passes; its full unfiltered
workspace/release gate remains open pending two separately scoped process-env
tests. Evidence is in docs/OPENSTORM.md; 0.1.36/0.1.37 failures remain preserved.

The `0.11.1-openstorm.128` candidate preserves caller types in required anonymous
component `@props` and applies defaults only for missing/null values. Explicit
PHPDoc contracts remain authoritative. The existing `@aware` fallback is retained;
broader parent-component inference remains unaccepted. Patch128 replays through
the pinned recipe and its reconstructed source matches the working implementation.
The full source suite passes 18,388 tests (17 ignored), plus its separate FIFO
child. All-target Clippy is warning-free; formatting, fingerprint, production
build, 12 PHP example lints and runtime assertions pass. Updated Laravel13.37
example analysis reports exactly its three intentional errors. SDK/native50
acceptance is tracked in `docs/OPENSTORM.md`;127 and prior apps remain preserved.

Patch0132 makes providers within the same registration group follow their list
order, including parent/child providers and aliases. Application registrations
retain precedence over framework/package defaults. The pinned Laravel example
checks both orders through `Application::register` without booting the app.
Create the reverse-order client fixture with
`scripts/create-openstorm-container-fixture.py --reverse-providers` and the
installed132 Laravel13.37 baseline; set `OPENSTORM_TEST_CONTAINER_ORDER_ROOT` to
the returned project. The ignored actual-client regression covers exact key and
alias targets, hover, method completion/navigation, unsaved provider-list changes,
restart and closing the overlay. Contextual/environment-dependent registration
and broader facade snapshots remain outside this slice. Source, checksum and
packaged native acceptance are recorded in `docs/OPENSTORM.md`.

Patch0133 refreshes cached facade signatures and member candidates after a
provider binding changes. The completion cache rejects publication by an older
resolution generation. Use `--facades` with the container fixture generator and
set `OPENSTORM_TEST_FACADE_ROOT` to its project for the actual-client case. It
checks key/alias-backed facade hover, UTF16, method destinations and a method
present only on the replacement through unsaved edits, undo/redo, restart and close. Framework/source
and native acceptance remain distinct in `docs/OPENSTORM.md`.

Patch0134 preserves ContentModified instead of returning an empty declaration
while semantic publication is pending. OpenStorm retries these explicit replies
for definition/declaration/type-definition/implementation within one unchanged
deadline and source receipt. Typing, open/close, observed disk changes, settings
reconfiguration, restart and cancellation retire the request; genuine empty
results and unrelated errors are not retried. The pivot client fixture accepts
`--pin` for candidate source verification. Its regression navigates before hover
through dependency-only edit/undo/redo/restart/close with an unchanged consumer.
Source/client/build evidence and the unavailable native UI gate remain separate
in `docs/OPENSTORM.md`.

Patch0135 recognizes controller-group Route::match action strings by their third
positional argument or explicit named action. Shared facade/argument identity
keeps URI strings and foreign receivers separate and refuses unpacked/duplicate
ambiguity. Completion retains the existing second-argument fast path, reuses the
whole-identifier UTF16 edit, and adds AST selection for other supported layouts.
Focused source cases cover positional/named forms, nested owner hover/references
and invalid contexts. Full source18424ordinary/17ignored plus FIFOchild, fmt,
all-target release Clippy, source fingerprint, build and example/analyzer controls
pass. The real OpenStorm client verifies exact targets/refactor edits, completion
ranges and dependency-only unsaved/restart/close freshness. Remaining broader
route forms, named-URI metadata and native acceptance are tracked separately in
docs/OPENSTORM.md; development wiring alone does not establish final delivery.

Patch0136 selects URI metadata from an explicit named uri or its correct
positional slot for supported get/match/view/redirect registration methods,
including mixed layouts. Unpacking, duplicate bindings, missing URI and invalid
positional-after-named arguments retain unknown metadata. Public route discovery
regressions preserve prefixes/Unicode/optional parameters and negative guards.
Fresh18426ordinary source tests/17ignored plusFIFOchild, releaseClippy/fmt/
fingerprint/build and plain/Laravel examples/exact analyzer control pass. Actual
OpenStorm client baseline135 red and136 green verify typed list/registration
navigation and parameter completion through URI-only unsaved changes/restart
and restoration, preserving the caller and all3owned disk originals. Full SDK/
client/build acceptance and native limitations remain tracked in docs/OPENSTORM.md.

Patch0137 follows known owner-preserving RouteRegistrar modifiers for controller
actions inside groups. The shared AST action selector/extractor covers fluent
HTTP registrations and uses the existing completion replacement edit. Unknown
macros/receivers, owner-changing controller/namespace chains and ambiguous action
bindings stay unresolved. Inline controller registrations need separate owner
analysis. Fresh18428ordinary source tests/17ignored plusFIFOchild, releaseClippy/
fmt/fingerprint/build, updated examples/275Laravel runtime assertions and exact3
intentional analyzer diagnostics pass. Source and actual-client red/green,
example LSP definitions/completion edits/hover and final OpenStorm acceptance are
tracked in docs/OPENSTORM.md. Native checks remain explicitly user-skipped.
