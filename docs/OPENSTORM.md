# OpenStorm implementation and acceptance

Base: thinkycz/openidea 50212e73972d6f05bf16050cb0ba10102a6c8054 (0.1.21).
Branch: codex/openstorm-php-laravel. Target: macOS Apple Silicon, local delivery.

## Contract

Preserve OpenIDEA layout, default keymap, autosave and debugger. Use separate
cz.thinky.openstorm application data; retain .openidea project settings.
Bundle released PHPantom 0.11.1-workbench.114; support builtins and PHP target context.
The current scope is the full 69-area roadmap in `docs/openstorm-features.json`,
with Laravel/Blade/workflow work authorized in milestone order. New debugger
functionality, upstream automatic updates, public app releases, notarization,
actual profile migration and user-server deployment remain deferred. Selective
settings-import functionality may be implemented without migrating real profiles.

## Upstream synchronization (2026-10-29)

User-authorized scope: commit/push the existing work and merge the upstream
repository into this fork. `origin` remains `thinkycz/openidea`; `upstream` is
`l0ng-ai/openidea`. The working branch remains `codex/openstorm-php-laravel`.
The foundation was checkpointed in commit `3149758` before merging.

Upstream pin: `200ed6a52835e9d66923a2b4f5ad53f420afd24c`, 69 commits after
`50212e7`, 804 changed files, through upstream 0.1.24. OpenStorm's independent
source version is now 0.1.23; no new native app is built or installed by this sync.
The 0.1.22 package/receipts below describe the earlier foundation candidate.

Imported changes include editor navigation, folding, tabs/splits and LightEdit;
Eclipse keymaps and cancellable background tasks; multi-repository Git workflows
and diff navigation; database grid/filter/SQL improvements; native Codex/Claude
agent drivers, ACP v2/registry and retry reporting; proxy settings, system fonts,
mise/asdf toolchains and remote environment reload; plus Finder file-paste,
macOS Settings-window visibility and WebKit compatibility fixes. Existing upstream
debugger improvements are inherited, not newly accepted OpenStorm roadmap work.

Integration preserves OpenStorm identity/caches and attribution, disabled updater
feed/artifacts, bundled PHPantom and its checksum, `.openidea` compatibility,
save-conflict prompts and guarded edit plans. PHPantom's checked rename takes
precedence over upstream multi-server rename so a refusal cannot be masked by an
unchecked provider; the new regression covers that combination. New upstream
workspace-edit fixtures were adapted to the optional guard field. Source-text
translations and updater strings retain OpenStorm branding.

The existing Unicode regressions exposed different behavior of an end-anchored
Unicode character class in local Node 26 versus Node 24. A shared code-point
boundary check preserves UTF-16 offsets and old-WebKit compatibility; both existing
regressions passed after the adjustment. Dependency setup uses upstream's pnpm 11
workspace format (local validation 11.28.9), without changing global developer tools.

Verification: frontend 2,986 passed; browser E2E 21 passed / 1 existing skipped;
type checking, lint, format check and frontend build passed. Lint retains inherited
warnings. Rust workspace: 1295 passed, 0 failed, 24 ignored.
Workspace Clippy (`-D warnings`) and Rust formatting passed. The real PHPantom
client cases (2) and actual-host checked-edit case (1) also passed after integration.
Local Rust tests use `cargo test --workspace --no-fail-fast`, not CI's nextest;
Linux/Windows/musl CI and post-sync native acceptance are not claimed.

## Foundation batch status (2026-10-29)

Batch 1 is implemented and packaged as a locally signed development build at
`artifacts/batches/0.1.22/OpenStorm.app`. Final native acceptance is partial: the
latest package needs a controlled final-copy recheck and quit/relaunch check after
computer-use controls reported user activity changes. No dependent Laravel
navigation batch has started. The installed optimized 0.1.21 app is preserved.

Final source checks: 1,101 Rust workspace tests passed, 0 failed, 19 ignored;
8 focused document transaction checks passed; workspace Clippy and formatting
passed. Frontend: 2,536 passed, type checking/lint/build passed with the existing
two lint warnings. Real engine client and actual-host edit regressions passed.

The pinned source was freshly built with its locked dependencies and 114 verified
patches. The rebuilt executable passed both client cases and the host checked-edit
case. Its receipt is `artifacts/checks/phpantom-rebuild-receipt.json`. The shipped
released executable remains unchanged; rebuilt byte identity is not claimed.

## Foundation delivery progress (0.1.21, historical)

- OpenStorm application name, cz.thinky.openstorm identity and isolated updater override implemented.
- PHPantom is bundled with original licenses, checksum and source/patch provenance; build verifies the checksum.
- Default PHP server resolves the bundled engine; explicit custom paths remain supported. Intelephense and Phpactor remain fallbacks.
- Builtin class/function documents use the engine protocol, validate returned identity and show the applied PHP target.
- Frontend: 299 files / 2,529 tests passed; typecheck and build passed. Lint passed with two existing React library warnings.
- Rust LSP/tools checks: 147 passed; environment-specific tests were reported separately.
- Real engine through LspManager: Composer/autoloaded helpers, completion import edits, hover/signature/definition/references/two-file rename, unsaved member completion, diagnostics, builtin target and restart/shutdown passed.
- Laravel 12 fixture: model completion, Eloquent builder completion and model methods after query()->where()->first() passed.
- Full workspace clippy with `-D warnings` passed; Rust formatting passed.
- Native release build completed and installed at `/Users/longdo/Applications/OpenStorm.app` (about 181 MiB).
- Local ad-hoc signing verified before and after runtime tests. The bundled engine checksum stayed unchanged.
- Full workspace tests: 1086 passed, 0 failed, 17 ignored; existing PhpStorm EAP JBR 25 supplied the Java fixture.
- Both real-engine tests passed again against the executable inside the installed application.

## Packaged acceptance

| Scenario                               | Observed result                                                                                                                                                                                             |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Clean-profile launch and app identity  | Passed; settings/index/history under the disposable profile's `Library/Application Support/cz.thinky.openstorm`, logs under its `Library/Logs/cz.thinky.openstorm`                                          |
| Bundled engine without a global server | Passed with PATH restricted to system directories; process executable was inside OpenStorm.app; version .114                                                                                                |
| Plain PHP                              | Method completion, insertion and parameter help passed without Composer                                                                                                                                     |
| Composer PHP                           | Method completion, snippet parameters, class completion with import insertion, call documentation, declaration navigation, usages, two-file reviewed rename and diagnostics passed                          |
| Unsaved cross-file state               | New class method completed in another buffer while its defining file on disk still lacked the method                                                                                                        |
| Builtin source                         | DateTime opened read-only with PHP 8.5 target banner; attempted typing did not modify it                                                                                                                    |
| PHP target                             | Composer ^8.2 target reported by the engine; ^8.2 also verified through builtin navigation in the integration test                                                                                          |
| Laravel 12                             | Custom model method completed directly and after `query()->where()->first()`; builder methods passed the real-engine test                                                                                   |
| Default autosave                       | Saved completion/rename edits to disk on focus change; default checkbox verified; restored after temporarily disabling it for unsaved/conflict tests                                                        |
| External clean changes                 | Reloaded external text and cleared obsolete diagnostics                                                                                                                                                     |
| External dirty conflicts               | Fixed: autosave pauses; explicit Save asks Overwrite / Cancel (default). A newer disk edit invalidates earlier consent. Native cancellation, overwrite, reopened-tab retention and quit cancellation passed |
| Server restart                         | Process ID changed, open files reopened, diagnostics continued                                                                                                                                              |
| Quit/relaunch                          | App and PHP service exited; no-argument relaunch restored the Composer project, tabs and saved content                                                                                                      |
| Layout, settings and terminal          | Rounded upstream layout inspected; settings shortcut, Go to File, Go to Line, declaration, documentation, usages, rename and terminal tab controls exercised                                                |
| Debugger                               | Existing implementation retained; no runtime debugger acceptance claimed                                                                                                                                    |

Laravel and Composer disposable copies were corrected for inherited namespace
escaping and their autoload files regenerated with project scripts/plugins disabled.
Original evaluation artifacts were preserved. Laravel tracing confirmed all four
`dis` completion candidates, including displayName; the unique `displ` prefix
inserted the method in the native editor. A temporary stdio tracing wrapper was
removed from the test profile; the final plain-PHP run used default bundled resolution.

## Known limitations

- Quick documentation worked on the call site; the tested declaration position returned no documentation.
- The local toolchain emitted a missing-libLLVM warning while stripping debug information. The release build succeeded; app size is not a final optimized distribution-size claim.
- Local ad-hoc signature only; no notarization, publication, cross-platform or new debugger validation.

## PHP configuration

The engine reads Composer PHP requirements and existing `.phpantom.toml` project
configuration. Builtin source banners show the applied target returned by the engine.
Existing Language Servers settings can disable PHPantom or override its command/args;
Tools settings can override `phpantom_lsp`. No global installation is needed on the
supported platform. Existing Xdebug configuration is unchanged.

## Verification commands

- `pnpm test`, `pnpm typecheck`, `pnpm lint`, `pnpm build`
- `cargo fmt --all --check`
- `cargo test -p ide-tools -p ide-lsp --lib --tests`
- `OPENSTORM_TEST_LARAVEL_ROOT=<Laravel fixture> cargo test -p ide-lsp --test phpantom -- --ignored`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `pnpm tauri build --bundles app`

Native test cases use disposable copies in `/tmp/openstorm-acceptance`.
PHP Workbench and the original evaluation checkout are preserved.

## Build environment

macOS 27.0.1 (26A434), Apple Silicon; rustc 1.99.1, Node 26.7.0, pnpm 9.15.0.
Rust uses the preserved toolchain and shared build cache under the earlier evaluation
workspace. Local check logs are preserved in `artifacts/checks/` (ignored by Git).

## Delivery receipt

Installed bundle: `/Users/longdo/Applications/OpenStorm.app`, identifier
`cz.thinky.openstorm`, version 0.1.21, arm64. App/service processes were closed
after testing. `artifacts/checks/build-receipt.json` records executable hashes
and the source snapshot. No commits were pushed or releases published.
The original Java test failure and its successful JBR-enabled rerun are both
retained in local evidence. CI uses nextest; the local full-suite run used
`cargo test --workspace --no-fail-fast` instead.

## Save conflict follow-up (2026-10-28)

The overwrite regression failed on the previous build. The save path now compares
raw content fingerprints at write time, including same-size changes and deleted
files. Autosave pauses on conflict. Explicit Save offers Cancel (default) and
Overwrite; consent applies only to the reviewed disk revision. A newer external
change requires another confirmation. Cancelled Save All propagates to the quit
guard, and conflicted buffers survive closing their last tab.

Frontend: 2,536 passed. Rust workspace: 1089 passed, 0 failed, 17 ignored. Type checking, frontend build and formatting passed; lint retained two existing warnings. Rust clippy with `-D warnings` passed. Updated signed app installed and native acceptance passed.

### Save conflict packaged acceptance

- Default autosave preserved the external bytes and kept the editor dirty.
- Save showed Overwrite / Cancel, with Cancel focused by default.
- Cancel preserved both the external file and unsaved editor contents.
- Closing/reopening the last tab recovered the conflicted editor buffer.
- An external edit made while the dialog was open caused a fresh confirmation; outdated consent wrote nothing.
- Confirmed overwrite saved the editor version and cleared the dirty state.
- Quit → Save All → Cancel overwrite kept the application open with its dirty buffer and preserved external bytes.
- Successful save then quit exited normally; post-run deep/strict signature verification passed.

The backend now exposes `doc_save_checked(doc, overwriteRevision)` for editor
saves. The old save entry points reject conflicts, so non-editor callers cannot
silently bypass the guard. The previous installed app is recoverable at
`artifacts/OpenStorm-before-save-conflict.app`.

## Full 69-area roadmap continuation

The active requirements ledger is `docs/openstorm-features.json`. Workbench
completion claims are retained only as historical context, not OpenStorm acceptance.
Milestone order: PHP intelligence → Laravel understanding → Blade/frontend →
advanced transformations → integrated workflows → workbench acceptance → remote/AI.
FEAT-065–069 apply throughout; new debugger work remains user-deferred.

Current batch: guarded edit plans, PHPantom checked rename, Optimize Imports.
Status: implemented and packaged; final native acceptance remains partial (see current status).

### First batch implementation evidence (0.1.22 candidate)

- Guarded plans bind the open-buffer revisions, PHP project corpus/configuration,
  effective settings and engine generations. Application rejects stale previews,
  aliases, overlapping/colliding edits, invalid UTF-16 positions, read-only targets,
  targets outside editable roots and environment files.
- Every text target is staged before writing. Open overlays remain locked through
  commit. Original disk bytes and a manifest are retained before writes; on failure,
  rollback only restores files whose bytes still belong to this operation. Recovery
  copies remain available if rollback cannot safely restore a file.
- PHPantom checked rename includes exact UTF-8 SHA-256/version receipts for open
  overlays and transient project candidates. Missing candidates and scope limits
  refuse the operation. Intelephense/Phpactor retain standard rename.
- Existing Optimize Imports invokes the advertised PHPantom extension, verifies its
  returned source receipt, and refuses edits to another document. Existing menu,
  search, keymap and save-participant entry points are retained.
- Source recipe: `engine/README.md`, `engine/source.json`, 114 verified patches.
  Archive acquisition/patch replay/locked Cargo metadata passed; the bundled engine
  has not been replaced or rebuilt.
- Focused regressions observed: 8 document transaction cases, 4 PHP client protocol
  cases, and 2 real-engine client integration cases passed. Broader gates and native
  acceptance for this candidate are still pending.

Limits to verify: multi-file filesystem writes are recoverable, not an OS-level
transaction; an external writer can still race an individual atomic replacement.
The current conservative project fingerprint scans PHP source (including vendor)
for each reviewed request; performance is not yet accepted. Reviewed file creation,
move/delete and migration of other generated-edit entry points remain outstanding.

### 69-area acceptance inventory

The complete original acceptance requirements are retained in
`docs/openstorm-features.json`; each row below remains visible until individually
accepted or explicitly deferred. “Partial” does not transfer Workbench acceptance.
FEAT-058/059 use explicit OpenStorm acceptance overrides in the inventory: the
OpenIDEA appearance and docking take precedence over the old Workbench layout.

| ID       | Area                   | Feature                                                | Status        |
| -------- | ---------------------- | ------------------------------------------------------ | ------------- |
| FEAT-001 | Editor                 | Buffers, files and encoding                            | partial       |
| FEAT-002 | Editor                 | Multiple carets and selections                         | partial       |
| FEAT-003 | Editor                 | Syntax highlighting and mixed documents                | partial       |
| FEAT-004 | Editor                 | Tabs, splits and workspace recovery                    | partial       |
| FEAT-005 | Editor                 | Find/replace and project search                        | partial       |
| FEAT-006 | Editor                 | Folding, breadcrumbs, outline and minimap option       | partial       |
| FEAT-007 | Editor                 | Smart indentation, pairs and snippets                  | partial       |
| FEAT-008 | Editor                 | Large files and resource limits                        | partial       |
| FEAT-009 | Editor                 | IME, Unicode and accessibility                         | partial       |
| FEAT-010 | Editor                 | Local history and recovery                             | partial       |
| FEAT-011 | PHP intelligence       | Contextual PHP autocomplete                            | partial       |
| FEAT-012 | PHP intelligence       | Expected-type and name ranking                         | partial       |
| FEAT-013 | PHP intelligence       | Hover and signature help                               | partial       |
| FEAT-014 | PHP intelligence       | Definition/implementation/type navigation              | partial       |
| FEAT-015 | PHP intelligence       | References and usages                                  | partial       |
| FEAT-016 | PHP intelligence       | Rename                                                 | partial       |
| FEAT-017 | PHP intelligence       | Diagnostics and intentions                             | partial       |
| FEAT-018 | PHP intelligence       | Format and import optimization                         | partial       |
| FEAT-019 | PHP intelligence       | PHP version and extension profiles                     | partial       |
| FEAT-020 | PHP intelligence       | Composer project understanding                         | partial       |
| FEAT-021 | PHP intelligence       | Flow, generics and shapes                              | partial       |
| FEAT-022 | PHP intelligence       | Extract/inline/change-signature refactorings           | partial       |
| FEAT-023 | PHP intelligence       | Generate constructors/accessors/overrides/docs         | partial       |
| FEAT-024 | PHP intelligence       | Call/type hierarchy and structural search              | partial       |
| FEAT-025 | Laravel                | Framework/version detection                            | partial       |
| FEAT-026 | Laravel                | Eloquent model fields and casts                        | partial       |
| FEAT-027 | Laravel                | Eloquent builders, scopes and collections              | partial       |
| FEAT-028 | Laravel                | Relationships, pivots and morphs                       | partial       |
| FEAT-029 | Laravel                | Facades and container inference                        | partial       |
| FEAT-030 | Laravel                | Routes and controller actions                          | partial       |
| FEAT-031 | Laravel                | Config, environment references and translations        | partial       |
| FEAT-032 | Laravel                | Views and cross-file data                              | partial       |
| FEAT-033 | Laravel                | Blade PHP + HTML/CSS/JS source mapping                 | partial       |
| FEAT-034 | Laravel                | Blade components, props, slots and directives          | partial       |
| FEAT-035 | Laravel                | Validation, FormRequest and auth policies              | partial       |
| FEAT-036 | Laravel                | Migrations and schema-aware assistance                 | partial       |
| FEAT-037 | Laravel                | Factories, seeders and generated resources             | partial       |
| FEAT-038 | Laravel                | Artisan panel and command runner                       | partial       |
| FEAT-039 | Laravel                | Laravel logs and exception navigation                  | partial       |
| FEAT-040 | Laravel                | Livewire, Alpine and Inertia                           | partial       |
| FEAT-041 | Laravel                | Pint, Larastan and Laravel coding actions              | partial       |
| FEAT-042 | Laravel                | Project dashboard, jobs/events and resource generation | partial       |
| FEAT-043 | Tooling and full stack | PHP interpreters: local/Herd/container/SSH             | partial       |
| FEAT-044 | Tooling and full stack | Composer commands and package UI                       | partial       |
| FEAT-045 | Tooling and full stack | PHPUnit and Pest test explorer                         | partial       |
| FEAT-046 | Tooling and full stack | Coverage and profiling                                 | partial       |
| FEAT-047 | Tooling and full stack | Xdebug stepping and variables                          | user-deferred |
| FEAT-048 | Tooling and full stack | Remote/container debugging                             | user-deferred |
| FEAT-049 | Tooling and full stack | Git status/diff/blame/staging                          | partial       |
| FEAT-050 | Tooling and full stack | Terminal and run configurations                        | partial       |
| FEAT-051 | Tooling and full stack | HTTP request files and response viewer                 | partial       |
| FEAT-052 | Tooling and full stack | Database browser, SQL and migrations                   | partial       |
| FEAT-053 | Tooling and full stack | HTML/CSS/JS/TS, Vue/React and Vite                     | partial       |
| FEAT-054 | Tooling and full stack | Tailwind classes and CSS assets                        | partial       |
| FEAT-055 | Tooling and full stack | Remote SSH/SFTP/deployment workflows                   | partial       |
| FEAT-056 | Tooling and full stack | Docker/Sail environments                               | partial       |
| FEAT-057 | Tooling and full stack | AI full-line/chat/agents and MCP                       | partial       |
| FEAT-058 | Workbench              | Islands Dark/Light/contrast and density                | partial       |
| FEAT-059 | Workbench              | Project/editor/tool-window docking                     | partial       |
| FEAT-060 | Workbench              | PhpStorm-familiar keymaps                              | partial       |
| FEAT-061 | Workbench              | Search Everywhere / action palette / recent files      | partial       |
| FEAT-062 | Workbench              | Settings and import/migration                          | partial       |
| FEAT-063 | Workbench              | Notifications, indexing and crash states               | partial       |
| FEAT-064 | Workbench              | Install/update and platform services                   | partial       |
| FEAT-065 | Quality                | Performance and memory                                 | partial       |
| FEAT-066 | Quality                | Versioned semantic snapshot correctness                | partial       |
| FEAT-067 | Quality                | Trust and execution boundaries                         | partial       |
| FEAT-068 | Quality                | Language service/process isolation                     | partial       |
| FEAT-069 | Quality                | Reference corpus and parity reporting                  | partial       |

### Broad gates for the first batch candidate

- Rust workspace: 1,096 passed, 0 failed, 18 ignored. Two real-engine client cases
  were then run explicitly and passed, including unsaved collision refusal.
- Frontend: 2,536 passed; type checking and production assets build passed.
- Workspace Clippy (`-D warnings`) and Rust formatting passed. Frontend lint
  has the two pre-existing React Compiler compatibility warnings.
- Local packaging uses the development profile for native iteration. This does
  not replace the installed optimized 0.1.21 bundle or establish release-profile
  performance. Native acceptance and promotion remain pending.
- Intermediate logs include a compile attempt invalidated by a revision-interface
  change and a Clippy style correction; the `*-final.log` files record the frozen
  candidate's successful broad checks.

### Native iteration findings

The first development bundle exercised Ctrl+Alt+O (unused-import removal),
Cmd+Z/Cmd+Shift+Z, Shift+F6 (three-file preview), Cancel, and a stale preview after
an external caller edit. Stale application wrote nothing and displayed an explicit
request for a new preview. A fresh plan changed the declaration and two callers,
preserved the external comment/Unicode text, and left an unrelated same-name
method unchanged. Restart and subsequent definition navigation worked.

This candidate exposed stale PHP diagnostics after the closed declaration changed:
the host did not send PHPantom disk-change notifications. Notification integration
and an actual-host regression are now being validated. These observations do not
accept the corrected candidate yet.

A new rollback regression also exposed the inherited in-place write fallback.
Reviewed edits now use a strict atomic writer; normal saves retain their existing
behavior. All seven focused transaction cases passed, including disk-target witness
checks, hard-link alias refusal, rollback after a later write failure, and retained
recovery originals/manifest. The failure was reproduced before the strict-writer
fix. Broader checks and a corrected packaged native pass remain required.

### Final batch 1 receipt and remaining acceptance

- Package: `artifacts/batches/0.1.22/OpenStorm.app`, Apple Silicon, development
  profile, identifier `cz.thinky.openstorm`, version 0.1.22. Ad-hoc deep/strict
  signature verification passed. No optimized app promotion or publication.
- Current-source checks: `batch1-workspace-final.log`, `batch1-document-final.log`,
  `batch1-clippy-final.log`, `batch1-fmt-final.log`, `batch1-package-final.log`.
  Frontend evidence: `guarded-frontend-tests-final.log`,
  `guarded-typecheck-final.log`, `guarded-lint.log` and the packaged asset build.
- Fresh engine recipe: acquisition and patch checks, cached-source integrity and
  deliberate tamper rejection, locked build, version, then actual-host and client
  regressions all passed. The existing rust-objcopy/libLLVM stripping warning was
  non-fatal. The acquired source still passed integrity checks after compilation.
- Reviewed plans now hash buffer identity/version/encoding properties without
  copying all open-buffer text; changing line endings invalidates a preview even
  when the text version does not change. Disk scans stream fingerprints and staging
  is limited to 8 MiB per file / 64 MiB original bytes per plan.
- Native observations on the corrected candidate: three-file semantic rename,
  readable preview, cancellation/stale refusal, preservation of external comments
  and Unicode, unrelated-symbol exclusion, and definition navigation without a
  server restart. Optimize Imports and editor undo/redo worked in the native UI.
  Save conflict still focused Cancel and preserved both versions after cancellation.
- Final-copy native recheck, approved overwrite, controlled quit/relaunch and full
  process cleanup remain unaccepted after the interrupted session. The development
  app may remain open on its disposable fixture; do not treat an interrupted UI
  action as a passed check. No real user profile was migrated.
- Cross-file undo is currently per open editor plus Local History checkpoints;
  a single global undo transaction is not accepted. PHP class/property/import-alias
  rename matrices and all other generated-edit entry points still need acceptance.
- PHP target/profile changes need a follow-up for cached builtin documents, and
  conservative source fingerprints still need performance/exclusion validation.
  No startup/whole-app memory/idle-CPU parity claim is made from the process snapshots.

The next dependent delivery stays behind the first batch's remaining native gate.
All 69 areas remain tracked individually; no PhpStorm parity or roadmap completion
is claimed.
