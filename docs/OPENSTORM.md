# OpenStorm implementation and acceptance

Base: thinkycz/openidea 50212e73972d6f05bf16050cb0ba10102a6c8054 (0.1.21).
Branch: codex/openstorm-php-laravel. Target: macOS Apple Silicon, local delivery.

## Contract

Preserve OpenIDEA layout, default keymap, autosave and debugger. Use separate
cz.thinky.openstorm application data; retain .openidea project settings.
Preserve the released PHPantom 0.11.1-workbench.114 foundation and its provenance.
The current local candidate uses rebuilt 0.11.1-openstorm.118 for safe JSON translation
and environment sources; preserve builtin documents and PHP target context.
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

## Post-sync native validation (2026-10-29)

The signed 0.1.23 development copy at `artifacts/batches/0.1.23/OpenStorm.app`
was launched directly with a disposable profile and a syntax-checked synthetic
Composer PSR-4 fixture. The installed optimized 0.1.21 app was preserved.
`artifacts/checks/native-0.1.23-build-receipt.json` identifies the clean merge
source; fixture paths and runtime output are in the adjacent fixture/runtime files.

Observed through the native UI and persisted fixture files:

- Optimize Imports removed the unused import; Cmd+Z and Cmd+Shift+Z restored and
  reapplied it. Cmd+S persisted the result.
- Shift+F6 preview found the declaration and two callers across three files,
  excluding an unrelated same-name method. Cancel wrote nothing. Changing a caller
  externally invalidated the preview; a fresh rename preserved that comment and
  Unicode text. All affected PHP files passed syntax checks.
- Cmd+B navigated to the newly renamed, previously closed declaration without an
  engine restart; Problems reported no diagnostics.
- Autosave persisted a changed text file when focus moved to another file.
- External-save conflict focused Cancel. Cancellation kept both the dirty editor
  text and external disk text. Quit / Save All / Cancel kept the app open. An
  external change made while the overwrite prompt was open invalidated approval
  and required a fresh prompt. Explicit approval of the current disk revision
  persisted the editor text.
- Controlled Cmd+Q exited both app and bundled engine; deep/strict signature
  verification still passed after runtime.

The run exposed orphaned background tasks: PHPantom reuses work-done tokens for
concurrent lookups, while the client replaced their task IDs without finishing the
old UI tasks. A failing regression demonstrated two IDs for one token. The client
now reuses the task ID and drops its previous cancellation handle before replacing
it. The regression also checks that cancellation reaches the server and the final
end notification completes the task. All 51 mock-server regressions passed.

Corrected 0.1.24 packaging, saved-session relaunch, and native task-cleanup checks
are in progress. This is normal saved-session recovery evidence, not acceptance
of crash recovery for unsaved drafts. The wider rename matrices, global cross-file
undo, builtin target-cache invalidation and corpus performance checks remain open.

### Corrected native candidate 0.1.24

`artifacts/batches/0.1.24/OpenStorm.app` is a separate Apple Silicon development
bundle with an ad-hoc local signature. Its build/source receipt is
`artifacts/checks/native-0.1.24-build-receipt.json`; native results are recorded in
`native-0.1.24-acceptance.json` and `native-0.1.24-runtime.log`.

The same disposable profile reopened its four saved tabs and selected conflict
file, showing the explicitly approved editor contents. The renamed caller and
closed declaration remained correct, PHP syntax passed, Find Usages returned two
callers, and code lenses showed three class / two method usages. No orphaned task
remained visible. Restart PHPantom from Language Servers reinitialized version
0.11.1-workbench.114 and reopened its overlays; Cmd+B then navigated correctly.
Cmd+Q exited app and engine; post-runtime deep/strict signature verification passed.
The installed optimized app remains unchanged. Saved-session relaunch is accepted
for this fixture; crash recovery of unsaved drafts remains unaccepted.

The first workspace test run hit an inherited JDWP temporary-port collision
(`Address already in use`). Its focused retry passed; the final full non-fail-fast rerun
passed: 1,296 tests, 0 failed, 24 ignored. Frontend 2,986 tests, type checking, lint (inherited warnings), packaged
asset build, Rust formatting and workspace Clippy passed for the correction.

The remaining native gate for the first guarded-edit batch is accepted on this
disposable fixture. Laravel static route/view navigation is the next batch.

## Laravel static navigation batch (static slice accepted)

Adds typed `phpWorkbench/routes` and `phpWorkbench/views` adapters, advertised
capability checks, request cancellation and IDE-coordinate locations. The wire
`incomplete` flag is retained, including when a subsequent selected-name reply is
complete. Route metadata retains URI, parameters and package origin; duplicate
registrations remain separate targets. Bounds outside 1–5000 are rejected locally.
Open PHP overlays and the engine generation are checked again after each query;
a changed unsaved dependency rejects the response.

Navigate → Laravel Routes / Laravel Views and the existing action search open a
searchable chooser using the established Dialog/Input/List components. A selected
logical name is resolved again, then the user selects a target. Closing the chooser
cancels the pending request. A project switch cancels/discards old results. This
initial entry point requires an open PHP document; it does not start a new server
from a Blade-only context. No Artisan, Composer or application boot runs from it.

Current evidence: three adapter regressions pass (capability refusal, bounded
incomplete/duplicate results and unsaved-dependency freshness); five chooser tests
pass (selection, incomplete notices, filtering, cancellation and stale responses).
A selection regression caught opening the active row instead of the clicked target;
the corrected handler now uses the clicked item. The released engine's actual
OpenStorm client test passed route listing, duplicate registration targets, bounded
incomplete results, dotted view resolution and invalid logical-path refusal.
Translations were added after the catalog tests identified the new UI strings.

The disposable Laravel 12 fixture copies dependencies and excludes environment
files. Its inherited incomplete completion scratch was replaced in this new copy;
all non-vendor PHP syntax and the `App\\` Composer autoload check pass. Existing
Workbench/evaluation fixtures remain unchanged. Build 0.1.25, broad checks and native menu/search/filter/target/cancel/reopen/quit
acceptance are recorded below.

### Laravel navigation 0.1.25 acceptance

Local app: `artifacts/batches/0.1.25/OpenStorm.app` (Apple Silicon development
profile, ad-hoc signature). Build/source receipt:
`artifacts/checks/laravel-0.1.25-build-receipt.json`. Native results and fixture
identity: `laravel-0.1.25-acceptance.json`, `laravel-0.1.25-fixture.json`; runtime
and relaunch logs use the same prefix.

Observed in this exact copy:

- Both actions appeared in the native Navigate menu and standard action search;
  the existing shortcut-assignment affordance remained available.
- Filtering `invoice` listed `invoice.show` and its `{invoice}` URI parameter.
  The dynamic prefix produced a visible incomplete notice, retained when resolving
  the selected name. Both duplicate registrations appeared. Keyboard and mouse
  selection of the second target opened `routes/web.php:5:83`.
- The view chooser resolved `invoices.show` to its Blade file. Existing Cmd+B
  navigation worked from a route's `'show'` action string to the controller method,
  and from the controller's view literal to the template.
- Two intentional duplicate-route diagnostics were visible. An initially genuine
  `getenv()` string/false argument error was corrected by casting the dynamic prefix
  in the synthetic fixture; external refresh then showed zero errors / two warnings.
- Cancel closed the chooser; navigation preserved source bytes. A query immediately
  during engine shutdown reported unavailable; closing/reopening after initialization
  recovered. A fresh query after restart succeeded. No background task was orphaned.
- Quit exited app and engine. Relaunch restored three tabs and the selected target
  caret; view lookup worked again. Final quit/process cleanup and post-runtime
  deep/strict signature verification passed. The installed optimized app is unchanged.

Checks: 2,991 frontend tests passed; type checking, lint (inherited warnings),
format checking and asset build passed. Rust workspace: 1,299 passed, 0 failed,
25 ignored, including the three new adapter regressions. Workspace Clippy
(`-D warnings`) and Rust formatting passed. The real-engine route/view client case
was explicitly run and passed; ignored real-service/platform cases remain separate.
The initial catalog failures and selection regression are retained in check logs.

This accepts the static browse/resolve/navigation slice on the disposable Laravel 12
fixture. FEAT-030 and FEAT-032 remain partial: semantic route rename, wider registration
forms, Laravel 13/version gates, cross-file view-data inference, native project-switch
cancellation, and large route/view corpus performance still need acceptance. An open
PHP document is required for this initial entry point. The next Laravel slice should
exercise inherited config/translation navigation and completion through the actual
client/native editor, then address integration failures before expanding workflows.

## Config, environment and translation navigation batch (0.1.27, native gate pending)

FEAT-031 is being reassessed through OpenStorm's inherited LSP client/editor path.
The actual-client regressions now cover config-key definitions, locale alternatives,
cross-file usages, nested and literal JSON-key completions, unsaved config/PHP-locale/JSON
changes, and absence of PHP declarations from JSON message text. The separately approved
two-key environment fixture covers completion, navigation and usages through the client.
Environment-value privacy and native environment-key acceptance remain open.

Native findings on signed 0.1.25 using a new disposable Laravel fixture:

- Cmd+B on a config key after Unicode text opened `config/billing.php`; native
  Find Usages from the declaration found both callers.
- Translation Cmd+B retained both locale definitions with `lang/cs` and `lang/en`
  paths, and selecting the Czech result with the mouse opened the right declaration.
  Config and translation completion inserted the full dotted key with intact quotes.
- Keyboard activation exposed an inherited chooser focus bug: the header pin button
  received focus before the result list, so Enter pinned results instead of opening
  the selected declaration. A failing regression reproduced the wrong initial focus.
  The List primitive now supports a preferred-focus marker, and the LSP chooser opts
  into it. The regression checks initial focus, ArrowDown/Enter selection and no pin
  action; the existing header-button test still passes. All 17 focused popup/list
  tests passed after the correction.
- The fixture was restored through the editor after completion checks and saved;
  app/server exited on quit. Broader acceptance must use the corrected 0.1.26 copy.

The corrected 0.1.26 development build was signed and exercised below. Frontend:
2,992 passed; type checking, lint, formatting and build passed (inherited lint warnings).
The installed optimized app and earlier evaluation artifacts remain unchanged.

### Further JSON findings and current engine candidate

The corrected 0.1.26 copy passed native Down/Enter selection of the English locale
and refreshed a closed translation's target from line 3 to line 4 after an external
edit. Native JSON-key completion preserved a key containing a space and Cmd+B opened
its JSON declaration. Find Usages from that declaration then exposed missing routing:
only the optional JSON server owned the file, so PHPantom was never asked.

OpenStorm now shares standard `lang/*.json` / `resources/lang/*.json` overlays with
configured PHPantom instances in declared Laravel projects. Only references use this
implicit bridge; PHP formatting, completion and rename remain excluded for JSON.
Regular user-configured JSON providers keep their existing capabilities, unrelated
JSON/projects are excluded, and old PHPantom instances receive no JSON overlay unless
they advertise `phpWorkbenchJsonTranslations`. JSON changes/close/restart use normal
client synchronization. PHP diagnostics from the implicit bridge are suppressed.
All 58 mock-server cases passed after capability gating and diagnostic filtering.

The real client test exposed a second, engine-level fault: PHP examples in a JSON
translation value became global PHP functions. The client routing is therefore not
accepted on the released 114 engine. A local engine candidate,
`0.11.1-openstorm.115`, adds patch 0115: JSON translations are non-PHP resources with
empty PHP symbol maps, current-source ownership and translation-key cache invalidation.
It advertises the safe-JSON capability. The patch adds private and handler regressions,
a changelog entry, and engine-local formatting rules matching its existing style.
The original 114 patch series is unchanged. Pinned replay and the rebuilt binary checks passed as detailed below. The candidate
is bundled in the signed 0.1.27 development app; native acceptance is still pending.

The user explicitly approved only `OPENSTORM_DEMO_LABEL=sample` and
`OPENSTORM_DEMO_MODE=local` in disposable `.env` fixtures. A real-client environment
helper case passed completion, declaration navigation and cross-file usages while
preserving the two-line file. The native fixture now has the same approved values;
no existing environment file was changed. Broader environment-value privacy and AI
exclusion acceptance remain open.

Repeated versioned builds filled the compiler cache. With no compiler running, older
regenerable incremental entries were pruned (about 20 GiB estimated); app bundles,
source snapshots, PHP Workbench and evaluation outputs were preserved. The receipt is
`artifacts/checks/compiler-cache-prune.json`. The first workspace run also hit an
inherited mock Godot startup failure; its focused retry passed. The final frozen-source
workspace run passed after the engine/resource changes, as detailed below.

The pinned 115 build completed (SHA-256
`8c5254b512627ac40289ce168cfdd295bc8e99ea75497c5b4a4d83035e24239d`).
All six actual-client cases and the actual-host checked-rename case passed, including
PHP-example isolation, JSON declaration usages, unsaved JSON/PHP locale keys,
checked PHP rename with a shared JSON overlay, Laravel inference and builtin docs.
The local bundled resource/provenance and tool version now identify this custom
build. The released 114 binary/licenses/provenance are archived locally and its
provenance is tracked under `engine/releases/`; earlier signed apps are unchanged.
Final workspace checks: 1,303 passed, 0 failed, 27 ignored; all-target workspace Clippy
with `-D warnings` and Rust formatting passed. The additional actual-host JSON-first
regression passed separately after this run: opening only a JSON translation starts
PHPantom and finds both PHP callers, including UTF-16 positions after a Unicode value.
The 58 mock-server checks include safe capability gating, JSON-only reference routing,
normal explicitly configured JSON providers, diagnostics filtering and unrelated paths.

Engine checks: release all-target Clippy and formatting passed; resource units (6)
and translation handler regressions (2) passed from the immutable pinned replay.
Compatibility checks also passed: YAML/XML resource navigation (9), and broader
Laravel translation keys, locale/package resources and Blade callers (41). The later
JSON-first host regression passed focused all-target Clippy and Rust formatting.
The preceding scratch copy also passed required Clippy fixes followed by formatting.
The source fingerprint was rechecked after the build. The inherited stripping step
still warns that its `rust-objcopy` cannot load `@rpath/libLLVM.dylib`; build and binary
checks succeeded. The full engine suite remains pending approval because inherited
tests create additional temporary `.env*` files outside the approved two-key scope.
Engine example lint with installed PHP 8.3.31 passed 10 of 13 files, including the
Laravel demo; three upstream examples require newer PHP syntax. Runtime assertions
and the dependency-backed Laravel example diagnostic gate remain unverified.

Current local candidate: `artifacts/batches/0.1.27/OpenStorm.app` (Apple Silicon,
ad-hoc signature verified, not notarized). Build receipt:
`artifacts/checks/laravel-config-0.1.27-build-receipt.json`. Its bundled engine is the
115 binary above. Runtime source matches the receipt; later changes affect only a host regression,
the progress/feature ledger and engine build documentation. The installed
0.1.21 app remains untouched. A fresh isolated profile and JSON-first fixture were
launched, but computer use reported the Mac locked before any 0.1.27 UI acceptance.
Unlock was requested; final native JSON/environment navigation, keyboard selection,
restart/relaunch, quit/process cleanup and post-runtime signature checks remain open.
The current running development app must not be promoted as accepted.

Evidence logs are under `artifacts/checks/`: `laravel-config-frozen-{workspace,clippy,rustfmt}.log`,
`laravel-config-final-format.log`, `phpantom-115-{resource-final,trans-integration,openstorm-client,openstorm-host}.log`,
`phpantom-115-{resource,translation}-compatibility.log`, `phpantom-115-example-lint.json`,
and `laravel-json-first-host-{final,clippy,format}.log`.
FEAT-031, FEAT-064, FEAT-066 and the other 65 ledger areas retain their current status;
this batch adds evidence without claiming full-area acceptance.

### Native gate blocked on desktop access

Computer use confirmed the Mac locked across three consecutive goal turns. The
remaining 0.1.27 native workflows cannot proceed until the user unlocks it. All
currently authorized command-line checks for this batch are finished; no native
acceptance, promotion or subsequent dependent batch is claimed. Source changes,
the signed candidate, disposable fixtures and evidence remain preserved. Resume
with the existing isolated 0.1.27 app/profile after unlock; the full engine suite
still needs the separately requested broader synthetic environment-fixture approval.

### Resumed native checks and environment declaration regression (2026-10-29)

The user unlocked the Mac and explicitly approved the full pinned regression suite's
suite-owned temporary `.env*` files with built-in fake data (including creation,
changes and cleanup); existing environment files remain outside that scope.
The full 115 source suite is now running from its unchanged verified snapshot.

Signed 0.1.27 native observations: JSON declaration Find Usages found three PHP
callers, result selection showed a preview and double-click opened the caller.
Environment helper Cmd+B opened the exact `.env` key; Ctrl+Space inserted the full
approved key with intact quotes, and Find Usages from PHP found three callers.
The locale chooser retained both paths and Down/Enter opened English line 4.
The PHP completion fixture was restored and saved. Save All quit exited both app
and server; relaunch recovered the saved session. Native PHPantom Restart changed
the child PID, and JSON declaration usages worked afterward. Final normal quit
again exited both processes. Deep/strict signature verification passed after runtime,
and the two-key environment checksum remained unchanged. Evidence:
`artifacts/checks/laravel-config-0.1.27-native-result.json` and the relaunch log.
A background relaunched window initially had no visible web content through computer
use; keyboard activation exposed its recovered editor and controls. This observation
is retained, without claiming a reproducible startup defect or a passed visual gate.

Find Usages from an environment declaration exposed a separate gap: no server owns
`dotenv` documents. The existing real-client environment regression now reproduces
that failure (`laravel-env-declaration-red.log`). In-progress 116 engine/client work
adds a separate safe environment-resource capability and a references-only bridge;
old engines receive no dotenv overlays. The 60 current mock-server checks passed,
including capability separation from JSON, diagnostics filtering, overlay change/close,
unsupported edit features, nested files and unrelated projects. Engine changes are
isolated in mutable scratch until source/client/native checks pass; 115 bundled and
archived artifacts remain the current packaged baseline. No 116 acceptance is claimed.

The full 115 source run completed all targets. Two integration assertions compared
macOS `/var` temporary paths with engine-canonical `/private/var` paths while
returning the correct helper definitions. The entire integration target rerun with
canonical `TMPDIR` and the crate source working directory passed 9,871 cases,
0 failed, 2 ignored. A first direct rerun used the wrong working directory and
therefore failed four relative Blade fixture lookups; that invocation is preserved
separately. Other source targets passed in the original Cargo run. The per-target
results and corrections are recorded in `phpantom-115-full-suite-result.json`;
source files were not changed to make these assertions pass.

The pinned 116 source replay and locked release build completed. Engine SHA-256:
`6f3ddc900bf78ad21121e59791acf04e1c8e41bfbb81c4cd3714d8b97532ebba`;
patch-series SHA-256: `1215ad50b01a5776801a53dacd1940dd9a980d5192ab13345a7974302b10f2a0`.
All six real-client cases passed, including the reproduced environment declaration
failure, unsaved key removal and close-to-disk recovery. Three actual-host cases
passed, including opening only a dotenv document and finding both closed PHP callers.
The source resources/provenance/tool version now use 116; the 115 app/archive and
tracked candidate provenance remain preserved. The full 116 source suite is running;
0.1.28 frontend tests/type checking/lint/build passed (existing warnings retained).
Native 0.1.28 acceptance has not started.

Installed PHP 8.3 could not parse three inherited engine examples. The current
StaticPHP 8.4.27 artifact endpoint returned HTTP 403; its documented older 8.4.23
macOS ARM64 archive downloaded successfully. Only its `php` executable was extracted
inside `artifacts/toolchains/`, with observed SHA-256 provenance (no independent
vendor SHA-256 is claimed). No system PHP/Herd settings or environment files were
changed. All 13 engine example syntax checks and runtime assertions then passed
using this workspace-only PHP 8.4.23 with `-n` (no existing php.ini loaded).

### Environment resources: 0.1.28 acceptance and follow-up (2026-10-30)

The full pinned 116 Cargo suite passed: 18,359 tests, 0 failed, 17 ignored (the
separate spawned bootstrap child output is not double-counted). Workspace checks
passed 1,305 tests, 0 failed, 29 ignored; all-target Clippy and Rust formatting
passed. Frontend: 2,992 passed, type checking/lint/build/formatting passed with the
inherited warnings. Engine Clippy passed; the required package `cargo fmt --check`
passed. An extra `--all` formatting attempt incorrectly crossed into the vendored
Tower LSP dependency's enclosing workspace; its failure is retained separately,
and no vendor/source change was made to work around it. Source integrity passed
again after the checks. The known rust-objcopy/libLLVM build warning remains.

The original Laravel example lock is 13.24.0. Static analyze correctly returned five
diagnostics: the three deliberate demo errors plus two unsupported higher-order
`sole` proxy properties. Both 115 and 116 returned the same five; the installed
framework's proxy list omits `sole`. A separate disposable copy was explicitly
updated to exact Laravel 13.37.0 with plugins/scripts disabled, matching the demo's
version-aware runtime oracle. It returned exactly the three expected intentional
diagnostics and passed all 267 runtime assertions using in-memory SQLite. Neither
fixture contains an environment file. Composer execution profiles and logs are
stored with the check artifacts; the original lock and system tooling were retained.

Signed development `artifacts/batches/0.1.28/OpenStorm.app` passed the native first-open
`.env` path on a fresh Laravel 13.37 fixture: no PHP server existed before opening
that file, then the bundled 116 process started and declaration usages found two
closed PHP callers. Default focus autosave persisted the first exploratory edit.
It was temporarily disabled through Settings in the disposable profile for the
unsaved checks, then restored to its original true; idle save remained false.
With both env/PHP disk files still original, completion followed an unsaved env-key
removal and reference search included an unsaved PHP caller. An external fixture
change produced the paused-autosave warning and an overwrite question on Cmd+S;
Cancel was focused, Return preserved external bytes and both unsaved buffers.
An explicit overwrite approval then restored only the synthetic fixture. Final
PHP/env data and autosave preferences were restored, normal quit exited app/server,
and post-runtime deep/strict signature verification passed.

A further native failure remains: Go to Definition on an indented `export` assignment
lands at column zero before the key, so an immediate Find Usages returns nothing.
The actual-client regression reproduced `0` instead of expected column `9`.
In-progress 117 scratch work shares key-offset parsing between definitions and
references, emits UTF-16 key positions, and strengthens both private and handler
regressions. It has passed required Clippy fixes followed by formatting; final
source/client/native checks remain open. 0.1.28 remains a development candidate
with this known failure, rather than an accepted full environment-navigation batch.
Evidence: `laravel-env-0.1.28-{build-receipt,native-result,fixture}.json`,
`laravel-env-export-destination-red.log`, `phpantom-116-full-suite.log`,
`phpantom-116-laravel13-37-{analyze,runtime}.log`, and the workspace/frontend logs.

A second cache prune removed only older inactive generated incremental entries
(252 directories, about 14.7 GiB estimated), freeing room for the versioned native
bundle. No signed app, source snapshot, PHP Workbench or evaluation result was
removed. Receipt: `compiler-cache-prune-0.1.28.json`.

The 117 correction passed scratch all-target Clippy and formatting, then replayed
from the pinned source with patch-series SHA-256
`13df68d2a64638e30a3e238bd6e14fac21d3d232c2983f2f6f1493ddab2e71ef`.
The locked release build produced engine SHA-256
`11b101f95b869af2cc88e7f4d48eeda227bab511b3da7ddf9774ddde01ec5f29`.
The stronger real-client export-destination regression passed, including reference
search directly from the returned destination. Three real-host cases passed. The
first Eloquent client invocation accidentally selected the new playground fixture,
which lacks the required custom `App\Models\User::displayName` model; its retry
against the existing acceptance fixture passed. The test requirement/error now names
that fixture explicitly, and a final complete client run uses the correct fixture.
The original failed invocation is retained. Full 117 source gates and native 0.1.29
checks remain in progress; no acceptance is inferred from Clippy or capability flags.
The 0.1.28 isolated profile's `settings.toml` was read back with `autoSave = true`,
and original PHP/environment bytes and process exit were verified after cleanup.

### 0.1.29 native resource-navigation gate accepted (2026-10-30)

The user's Mac-unlocked reply resumed native acceptance, and their explicit full
synthetic-fixture approval covered the pinned engine suite's owned temporary
`.env*` data. Existing user environment files were not changed. Engine 117 completed
18,360 tests with zero failures and 17 ignored; the bootstrap subprocess's one test
is counted once. All six real-client cases and three actual-host cases passed.
OpenStorm completed 1,305 workspace tests with 29 ignored and 2,992 frontend tests;
all-target Clippy, Rust/frontend formatting, type checking, lint and build passed.
The documented nonfatal engine build-script rust-objcopy/libLLVM warning remains.

The signed Apple Silicon development app at
`artifacts/batches/0.1.29/OpenStorm.app` passed a fresh dotenv-only open, starting the
bundled 117 engine and finding two closed PHP callers. With autosave temporarily
disabled in its isolated profile, unsaved environment and PHP edits drove completion
and usages while both disk files retained their original bytes. An indented export
now opens the exact key at UI 1:10; immediate Find Usages finds the expected two
callers, including the unsaved PHP buffer. Engine restart preserves those overlays.
Undo/redo changes navigation between the original 2:1 and exported 1:10 positions.
This supersedes the 0.1.28 exported-key failure above. JSON translation declaration
usages found three PHP callers; preview/double-click navigation and Down/Enter locale
selection opened the correct targets. Fixture PHP/environment bytes were restored.

Settings UI restored focus autosave to true; idle-save stayed at its false default.
Read-back of the isolated profile's settings TOML confirmed `autoSave = true`.
Normal quit exited app and engine. A controlled relaunch restored all five tabs,
selected file and layout; environment usages immediately found both PHP callers.
A second normal quit exited both processes. Post-runtime deep/strict signature
verification passed, and the bundled engine still matched SHA-256
`11b101f95b869af2cc88e7f4d48eeda227bab511b3da7ddf9774ddde01ec5f29`.
The installed optimized 0.1.21 app remains unchanged. This is a development build,
not notarized or published. Saved-session recovery does not establish dirty-buffer
crash recovery. Broader locale/helper/project changes, environment-value privacy/AI
exclusion and performance remain open; all 69 areas retain individual status.

Evidence: `artifacts/checks/laravel-env-0.1.29-{build-receipt,native-result,fixture}.json`,
`laravel-env-0.1.29-{native,relaunch,workspace,clippy,frontend-tests}.log`,
`phpantom-117-{full-suite,client-final,openstorm-host,final-clippy,final-fmt}.log`.

### Eloquent and facade acceptance slice on 0.1.29 (2026-10-30)

Extended the actual OpenStorm client corpus against the pinned engine Laravel
example, explicitly installed at exact framework 13.37.0. Eleven hover scenarios
verify exact boolean, float, Carbon, enum, nullable custom-cast, collection and
singular relationship types, a relationship method chain, an attributed protected
scope, an app-defined facade and a named container service. Six completion scenarios
verify scopes, retained model fields, relationship results and facade/container
methods. Three navigation assertions check both the target file and exact function
line. An unsaved model cast changes dependent hover from float to bool without a
disk write; closing that overlay restores float. All seven client cases passed in
the final combined run. Targeted Clippy with warnings denied and workspace Rust
formatting passed. The initial exploratory hover probe accidentally matched `bake`
inside `$bakery`; the corrected test uses an exact expression line/column. Its
failure log is retained and is not counted as a product failure.

The unchanged signed 0.1.29 runtime passed native F1 cast documentation (`float`
with cast origin), protected `freshlyBaked()` scope insertion, `quality float`
completion after `first()`, and Cmd+B to the exact protected declaration at
Bakery.php:74. The accepted completion was saved and its persisted PHP syntax
validated. Relationship documentation showed `Collection<Loaf>` with its origin;
facade Cmd+B opened BakeryService::bake at line 9. A native partial-text selection
was affected by folded/inlay offsets during test setup; that disposable edit was
undone, then full-document paste and explicit Go to Line positioning were used.
The final fixture is valid, model/service source bytes and approved environment
bytes are unchanged, normal quit exited app/server, and signature verification
passed again. No application code changed in this acceptance extension, so a new
bundle was not produced. Frontend and engine full-suite results from the matching
0.1.29 runtime remain applicable; this extension adds test code only.

FEAT-026–029 and FEAT-066 gain evidence, while their broader gates remain partial:
pivots, unknown morphs, contextual/environment container bindings and wider version
matrices remain open. Evidence: `laravel-eloquent-0.1.29-{native-result.json,
client-full.log,all-client-final.log,clippy.log,rustfmt.log}` in `artifacts/checks/`.

### Cross-file Blade view-data continuation (2026-10-30, in progress)

A new actual-client scenario verifies `compact()` controller data reaching Blade
hover/completion/definitions, a Unicode HTML prefix, unsaved controller and template
buffers, retiring the previous caller type, and a template-local assignment taking
precedence. Closed-expression checks passed on bundled 117. Immediate queries may
receive ContentModified while the semantic snapshot refreshes; the scenario retries
only this cancellation, then asserts the first successful result is current.

Native 0.1.29 opened the owned `openstorm-stock.blade.php` and displayed the Item
member type, then navigated to Item.php. With focus autosave temporarily disabled
only in the isolated profile, an unsaved controller change to Stock updated the
template: a closed expression completed `quantity` and documentation showed its
integer type. Fixture files were restored byte-for-byte, autosave restored to true
and read back, and normal quit exited both app and server.

Native editing exposed a real remaining failure: before typing the closing `}}`,
member completion falls back to HTML tags. The actual-client incomplete-echo case
also returns no members. The preprocessor retains incomplete expression text but
its source map puts the end caret after a generated terminator. A scratch 118
correction is being tested against this red case; no replacement binary or new app
is accepted yet. Logs: `laravel-view-data-client-settled.log`,
`laravel-view-data-incomplete-red.log`, `phpantom-118-map-red.log`; fixture receipt:
`laravel-view-data-native-fixture.json`. An earlier test invocation misspelled the
binary path; it is retained separately and is not evidence of an engine failure.

The private mapping regression confirmed the cursor was placed after
`echo e( $item->na); `, rather than after the member prefix. Scratch 118 records the
visible end and PHP expression end separately for unfinished escaped/raw echoes;
reverse anchors retain generated-suffix clamping. The correction adds a public
completion/edit-range regression for both echo forms and Unicode prefixes.
Required Clippy fixes followed by formatting and all-target Clippy passed, with
the existing nonfatal build-script stripping warning. Patch 0118 replayed against
the pinned upstream source; its new complete series hash is
`5ffbc565d9d3e5cace35ee78215f0ce06f9f278d2631f80f613a9fdf544716ff`.
A locked release build is running. Runtime promotion, green regressions, full engine
suite and a new signed native bundle remain open. Candidate 117 provenance/binary
and the accepted 0.1.29 app remain preserved.

Frontend follow-up completed with all 2,992 tests passing, type checking passing,
and lint passing with the existing warnings. Formatting and diff whitespace checks
passed. The engine release compiler remains live; resumable handle and next steps
are recorded in `artifacts/checks/phpantom-118-active.json`. Do not restart a live
build or promote the engine before the new real-client regression turns green.

The locked 118 build completed and its immutable archived binary has SHA-256
`d8a36840ba16625008a6fe85f25d3107f37d90c4230f97456aec608be6cfaefb`.
The previously red OpenStorm client scenario is now green, including completion
before the echo terminator. All eight real-client regressions and three actual-host
cases passed with that exact archived executable. Resources, provenance, tool
version and application metadata now target engine 118 / app 0.1.30; candidate 117
is retained under `engine/candidates/` and its immutable binary/archive is preserved.

Full pinned-source tests, final source Clippy/formatting, workspace tests/Clippy and
the new native bundle build are running with handles recorded in the active receipt.
The first app build was deliberately interrupted to correct metadata formatting and
restore the non-incremental compiler setting before continuing; its log is retained
as `laravel-view-0.1.30-native-build-interrupted.log`. The corrected build uses a fresh
log. The new native fixture has 95 lint-clean non-Blade PHP files, an unchanged
13.37.0 Composer lock, and only the two previously approved synthetic environment
keys. Native 0.1.30 acceptance and batch commit remain open; 0.1.29 is still the last
accepted signed app.

### 0.1.30 native follow-up: fix observed, further gate failures retained

Signed `artifacts/batches/0.1.30/OpenStorm.app` passed fresh-profile Blade-first
startup, unfinished escaped-echo completion, and unfinished raw-echo completion
with a same-line Unicode prefix. Native undo restores `pr`, redo restores `price`,
and saving after closing the raw echo persisted exactly `😀 {!! $item->price !!}`
plus newline. Diagnostics settled to no problems. With focus autosave temporarily
disabled, an unsaved Stock controller and unfinished template accepted `quantity`
while disk still held the Item controller and prior saved raw-price template.

The batch is not accepted: after engine restart, documentation/definition/completion
for that unsaved template variable were unavailable. A stronger actual-client case
reproduces this on first unsaved opens in the broad native fixture too. The minimal
fixture passes three opening/restart cycles; nested namespaces, a hyphenated view
name and configured custom view roots also pass. Adding the real vendor tree
reproduces the failure. Excluding only Composer's helper autoload manifest in a
separate owned fixture restores all cycles; using only Foundation/helpers.php can
reproduce on a later cycle. These are controlled fixture changes, not a proposed
product workaround. A same-offset caller variation still fails, ruling out a simple
one-character view-span shift. Stock itself resolves in the controller, while the
Blade variable hover is absent. Further helper/index interaction tracing is underway
in unpromoted scratch 119. No production change is inferred from these probes.

The original 118 source suite has one new test-contract failure: its member
completion assertion requires textEdit, but insertText with the protocol's default
word replacement is valid. The item itself is found. This assertion is corrected in
scratch 119; the original source/binary and failed log remain preserved. The suite
stopped at integration tests, so it is not reported as a full pass. Final source
Clippy and formatting passed independently. OpenStorm workspace checks passed
1,305 tests with 31 ignored, Clippy and formatting; this preceded the newest native
fixture regression. All 2,992 frontend tests/typecheck/lint and the app build passed.

Fixture data and synthetic environment hashes were restored/read back, focus
autosave restored true with idle-save still false default, and normal quit exited
app 41727 and restarted engine 45206. Post-runtime deep/strict signature verification
passed. 0.1.29 remains the last accepted signed app; 0.1.30 is retained as a failed
candidate. Evidence: `laravel-view-0.1.30-{native-result,unsaved-witness,fixture,
build-receipt}.json`, `laravel-view-data-{native-client-red,minimal-client-probe,
custom-roots-probe,no-autoload-files-probe,foundation-helper-probe}.log`, and
`phpantom-118-full-suite.log` in `artifacts/checks/`.

### Blade startup recovery: 119 regression and reproducible fixtures

The stronger native-fixture regression exposed repeated ContentModified cancellation,
not an accepted empty completion result. Tracing confirmed the current unsaved Stock
caller was parsed and inferred correctly, but startup input publication invalidated
the staged Blade scope. Its retained refresh was never scheduled again. Patch 0119
retries pending work when the input epoch changes during a drain; stable unsupported
components retain their existing non-looping behavior. A deterministic configuration
publication barrier test fails without the fix (pending work does not settle) and
passes with it. Scratch Clippy fix, formatting and all-target Clippy passed. The
prototype passes three actual-client open/restart cycles in both the broad native
fixture and the controlled full-vendor fixture. This is not packaged acceptance.

`scripts/create-openstorm-view-fixture.py` now creates a disposable, reproducible
view-data project/profile from the pinned Laravel example. It verifies 108 baseline
source files, exact Laravel 13.37.0 lock/install metadata and Composer autoloading,
excludes all environment files, then lints 88 non-Blade PHP files with the explicit
PHP CLI. The valid run passed; a missing baseline was rejected before fixture
creation. The generator executes no application/Composer code. No environment files
are created by it. Full synthetic engine-suite environment fixtures remain within
the user-approved suite-owned fake-data scope.

Compiler-cache cleanup preserved all source, fixtures, signed bundles and evidence;
only inactive generated incremental directories were pruned after checking active
compiler processes. Free disk space was observed at 20 GiB afterward. Source 119 is
now pinned for a normal optimized build; full-source, real-client and corrected
native app gates remain open. 0.1.29 remains the last accepted bundle.

The normal optimized 119 release build passed and reports the pinned version.
Archived SHA-256: `4940b94d33c7e5a02fbfe38bf18b7cb6e8bb0cd11d51ad930d1e5d623720db92`.
All nine actual-client regressions passed against that immutable executable,
including the broad generated fixture's three unsaved open/restart cycles.
Host/source/native gates remain open; no candidate acceptance is implied.

119 follow-up: all three actual-host cases passed. Workspace checks passed 1,305
with 32 ignored, all-target Clippy and formatting. Frontend passed 2,992 tests,
type checking, lint (inherited warnings) and build. The exact Laravel 13.37.0 corpus
still reports only its three intentional diagnostics at Demo.php lines 731/951.
An initial error-only severity invocation showed one because two diagnostics are
lower severity; the documented unfiltered invocation matches the baseline. Both
logs are retained. Native candidate 0.1.31 now bundles the archived 119 engine and
is building separately; the full source suite is still running.

### 0.1.31 Blade batch accepted locally

Signed development app: `artifacts/batches/0.1.31/OpenStorm.app`, bundled engine
119 SHA-256 `4940b94d33c7e5a02fbfe38bf18b7cb6e8bb0cd11d51ad930d1e5d623720db92`.
Native escaped/raw unfinished echoes complete before their terminator; Unicode raw
completion supports undo/redo and exact saved bytes. Both unsaved controller and
Blade changes resolve Stock quantity while disk retains the Item controller and
saved raw-price template. Actual server restart changes PID 39083 to 39541;
documentation, Cmd+B into Stock::$quantity and completion still work afterward.

Quit prompts for both unsaved files. Cancel retains them; Save All writes both
expected texts, then exits app 38202 and its server. Deep/strict signature verifies
after runtime. Relaunch without a project argument reopens the project and all
three tabs; quantity documentation still resolves. This checks saved relaunch,
not crash recovery. Native fixture restoration was saved; paste-added indentation
was removed only from the owned Controller file after final quit, returning all
four source files byte-for-byte. Focus autosave is restored true and idle autosave
remains false default. Final app/server exit and signature verification passed.
No environment files were created by this fixture.

Final engine source suite: 18,363 passed, 17 ignored, excluding the separate spawned
bootstrap child from the count. Final engine Clippy fix, formatting, all-target
Clippy and format check passed; clean-source fingerprint verification passed after
them. All nine actual-client and three host cases passed against the archived 119
binary. Workspace 1,305 passed / 32 ignored and Clippy/fmt passed before version bump;
afterward focused bundle-resolution tests (2), all-target workspace Clippy and
format check passed on 0.1.31. Frontend 2,992/typecheck/lint/build passed and native
build rebuilt frontend. Existing toolchain strip warnings remain non-fatal.

Observed first-IPC timings: 2,979 ms first-profile launch and 662 ms same-profile
relaunch. The first overlapped compilation; these are development IPC timings,
not normalized startup/UI-ready or parity measurements. See
`artifacts/checks/laravel-view-0.1.31-{build-receipt,native-result,unsaved-witness,
quit-witness,fixture}.json`, runtime/relaunch logs, `phpantom-119-*`, and
`laravel-view-119-*`. 0.1.29 and failed 0.1.30 bundles remain recoverable;
installed optimized 0.1.21 is unchanged. No publishing/push/notarization occurred.
All 69 areas remain visible and partial/deferred; broader Blade/component and
semantic freshness matrices continue after this accepted batch.

### Next view-data slice: nested includes

Accepted Blade recovery batch checkpoint: `27820e7` (local only). Working tree was
clean immediately after the commit. Next actual-client acceptance scenario follows
controller -> page -> partial, opening the leaf first, then changing the unsaved
controller model and the intermediate page binding. It checks mapped member goto,
retirement of previous members, restart and closing the intermediate overlay.
The first run is in progress against archived119; no nested-include acceptance is
claimed yet. No engine or application runtime code changed for this check.

Nested-include actual-client case passes against119; all ten real-client cases
pass together. Focused client-test Clippy and workspace formatting pass. The
runtime/source engine is unchanged from the accepted0.1.31 build, so its full
source/workspace/frontend/host/build gates are reused rather than rerun.

Native0.1.31 nested fixture: leaf-first Item name documentation resolves with both
providers closed. Unsaved controller Stock data completes quantity in the partial.
An unsaved intermediate Item binding overrides it, with Item name completion and
documentation. Server restart41043 ->41056 retains that binding. Closing the
intermediate overlay returns Stock quantity completion/documentation. All five
fixture source files remain byte-identical on disk throughout; no environment
files exist. Final normal quit discards only the owned overlays, exits app41016
and server41056, and post-runtime signature verifies. Focus autosave was restored
true only in the owned profile after quitting. Actual-client mapped member goto
passed; nested native goto was not separately exercised.

A separate editor gap was observed: with focus autosave disabled, closing the
modified intermediate tab silently loses the unsaved page binding. Reopening
shows disk text, and quitting reports three dirty files including that closed page.
Code inspection confirms DocumentManager.release disposes dirty models when
files.autoSave=false, and closeGuard retains docDirty ids after document close.
The next batch must guard individual/bulk tab closure and clear closed-document
dirty tracking, preserving default autosave and external-conflict protection.
This limits editor acceptance, not the observed include-data transitions.
Evidence: `laravel-include-119-client-{first,all}.log`, client-clippy/fmt logs and
`laravel-include-0.1.31-{fixture,unsaved-witness,native-result}.json`.
