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

Nested-include actual-client case passes against 119; all ten real-client cases
pass together. Focused client-test Clippy and workspace formatting pass. The
runtime/source engine is unchanged from the accepted0.1.31 build, so its full
source/workspace/frontend/host/build gates are reused rather than rerun.

Native 0.1.31 nested fixture: leaf-first Item name documentation resolves with both
providers closed. Unsaved controller Stock data completes quantity in the partial.
An unsaved intermediate Item binding overrides it, with Item name completion and
documentation. Server restart 41043 -> 41056 retains that binding. Closing the
intermediate overlay returns Stock quantity completion/documentation. All five
fixture source files remain byte-identical on disk throughout; no environment
files exist. Final normal quit discards only the owned overlays, exits app 41016
and server 41056, and post-runtime signature verifies. Focus autosave was restored
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

### Modified tab closure protection (next native candidate)

Nested-include evidence checkpoint `3ecbbbd` is local only. Native testing exposed
silent unsaved-tab discard when focus autosave is disabled and a stale quit dirty
count after closing a document. New controller regressions reproduced five missing
close prompts; separate regressions reproduced the retained conflict buffer after
explicit discard and closed-document dirty-count leak. All intended red logs are
retained (an initial test accessor typo was corrected before assessing the reds).

Tab-close actions now review the last visible copy of each modified document when
autosave is off. Cancel leaves tabs/buffers, Save uses the existing checked save
flow, and explicit discard leaves disk unchanged. Bulk closure reviews all targets
once; split copies are preserved. Deferred consent is rejected if tabs or document
revisions change. Existing default autosave remains on the unchanged path. Explicit
discard releases prior conflict retention; document-close events clear only that
id from quit tracking. Focused controller/document/quit tests passed 39 before the
last bulk-save/layout scenarios were added; final frontend/build/native gates remain
open. No engine changes are required; accepted 119 and 0.1.31 remain preserved.

Final focused tab-close/document/quit tests pass 41. Complete frontend passes 3,004,
with typecheck/lint/build passing. The final targeted rerun and typecheck also pass
after keeping the default-autosave path free of the new guard scans. New tests
cover explicit discard of a cancelled external conflict, delayed consent with newer
edits/tabs, bulk Save All, split sharing and default autosave. Native 0.1.32 is being
built with workspace tests/Clippy/fmt first, using CARGO_INCREMENTAL=0. Its isolated
plain-text/PHP fixture is `artifacts/checks/tab-close-0.1.32-fixture.json` with no
environment files. Resumable jobs and remaining gates are in the adjacent active
receipt. Do not infer packaged acceptance from these source tests.

0.1.32 workspace passed 1,305 tests with 33 ignored, all-target Clippy and formatting.
The native build is now running and independently verifies the unchanged119 engine
checksum before rebuilding frontend assets. No app/server is running yet. Full
PHPantom/client/host gates from the accepted119 runtime are unchanged and retained.

### 0.1.32 modified-tab protection accepted locally

Signed development bundle `artifacts/batches/0.1.32/OpenStorm.app` preserves the
accepted 119 engine checksum. Default Cmd+W still autosaves exact text without a
new prompt. With focus autosave disabled, close asks Save / Don't Save / Cancel;
Cancel keeps the modified tab and disk unchanged. A synthetic external edit then
Close -> Save uses the existing overwrite dialog with Cancel focused. Cancelling
keeps both versions and the tab visible. Explicit Don't Save subsequently closes
the buffer without writing disk, and normal quit has no phantom dirty prompt.

Relaunch without a project argument reopens the project; the discarded file opens
with external disk text. Closing one split keeps the shared modified buffer without
a prompt. Find Action discovers Close All Tabs; one prompt covers both modified
files. Cancel retains both tabs and prior disk versions; Save All writes both exact
new texts and closes both. Opening Note.php starts bundled 119. Focus autosave is
restored true, idle autosave stays false, and normal quit stops app 91082 / server 91124.

The original Laravel reproduction also passes in 0.1.32: modified intermediate Blade
close via Cmd+W and the X button asks first. Cancel retains the unsaved Stock binding;
explicit Don't Save closes it. All five project source files stay byte-identical.
Quit has no phantom dirty prompt and stops app 91187 / server 91195. The temporary Blade
profile's explicit manual-save setting was restored true after quitting. Deep/strict
signature verification passes after both runtime checks. Plain fixture sources were
restored byte-for-byte after their successful persistence checks. No environment
files were created or changed by these native fixtures.

Complete source gates: frontend 3,004 tests/typecheck/lint/build, final focused 41
editor/document/quit checks, workspace 1,305 passed / 33 ignored, Clippy/fmt and native
build passed. The unchanged 119 engine source 18,363 passed / 17 ignored, actual-client 10 and
host 3 gates are retained from the preceding accepted build. No new engine claims
are inferred from UI event activity. Evidence: `tab-close-0.1.32-{build-receipt,
native-result,fixture,blade-fixture,conflict-witness,bulk-witness}.json`, native logs,
`tab-close-guard-*` and workspace/build logs in `artifacts/checks/`.

A short 4-second idle sample on the tiny 4-file PHP/text project recorded native app
RSS 210.56 MiB / CPU 1.70% and engine RSS 49.33 MiB / CPU 0.00%. This excludes WebKit/GPU/
network helpers and is not total app memory or a parity benchmark; raw samples and
limits are in `tab-close-0.1.32-idle-sample.json`. Startup first-IPC observations are
in the native result receipt, separate from UI-ready latency. Broader performance,
crash recovery and editor matrices remain unaccepted. Previous 0.1.31 and installed
optimized 0.1.21 remain preserved. No push, publishing or notarization occurred.

### Policy / FormRequest acceptance slice started

Tab-close protection checkpoint `959f771` is local only; committed runtime files
were byte-checked against the signed0.1.32 build receipt. New actual-client policy
checks pass ability completion (excluding hooks/protected methods), precise gate /
convention-policy / registered-policy navigation and policy documentation.
FormRequest field goto reaches the exact rule declaration, but field-key hover
returns no result. The original failing expectation is retained in
`laravel-policy-119-client-identify.log` and the complete new test in
`laravel-policy-client-with-field-hover.rs`. A temporary probe limits hover checks
to policies to reach the remaining validation/unsaved checks; this is not field
hover acceptance. Engine119 has no FormRequest-key hover hook. Investigate and add
that missing documentation through the engine's existing validation-rule metadata;
restore the full field-hover assertion before accepting a replacement.

The field-hover public regression fails on119 and passes in scratch120. Four
focused checks pass: direct/inherited rule documentation, same-line Unicode UTF-16
literal range, refusal of unknown/unrelated keys and retained uncertainty. The
parser preserves computed expressions as source; new literal-value metadata lets
hover label those as not evaluated. Unknown rule keys retain an incompleteness
qualifier. Navigation and documentation share one resolution and ancestor origin.
All29 request-key regressions, scratch Clippyfix/fmt/all-target Clippy passed.

The temporary probe restriction is removed. All11 actual-client cases pass against
the archived opt0 prototype, including unsaved field-hover updates, restart recovery
and retiring old documentation after closing the rule overlay. All3 host cases also
pass. Seven extracted actual-client PHP caller variants passed explicit PHP -n -l
on stdin without execution. Final optimized source/binary and native0.1.33 gates
remain open; resources and accepted0.1.32 still use119. Inactive incremental
compiler caches were pruned after checking no app compiler used them; all binaries,
source, receipts, signed apps and earlier evaluation artifacts were preserved.

Final scratch checks pass after all five public hover cases and30 validation-rule
unit checks. The diagnostic per-package opt0 profile is removed. Patch0120 changes
nine reviewed source files, including a short usage comment in the Laravel example;
clean acquisition replays the series with SHA-256
`d8724ece5749022374bad91de25c9d0c81dd46c64d3cc9126e0caef81f7ae5b4`.
Normal optimized build is running. Fresh installed corpus
`artifacts/checks/engine-laravel13-37-120/project` preserves the exact13.37.0 lock and
vendor tree, updates only the pinned Demo comment, verifies108 source fingerprints
and lints85 PHP files. Original116 corpus and every accepted engine/app remain
preserved. Corpus copying excludes all environment files. Remaining optimized,
source and native gates are in `artifacts/checks/phpantom-120-active.json`.

Optimized120 build passed, archived SHA-256
`4db75680a66966955ed6d5aa43b92301f40d030383109bab80ed9deb790b38c2`.
All11 actual-client and3host scenarios pass against this exact executable and the
fresh pinned120 Laravel13.37 corpus. Unfiltered static analysis retains only the
three intentional Demo diagnostics at731/951. Frontend3,004/typecheck/lint/build
passed. Full engine source suite/final lint and root workspace/native0.1.33 gates
are running. Resources/provenance/version now target120/0.1.33 as an unaccepted
candidate; accepted0.1.32/119 remain preserved. Candidate119 provenance is retained
under `engine/candidates/`. The native fixture is linted, fingerprint-checked and
records exact1-based Demo locations for field/policy navigation. Remaining native
steps and active job handles are in `artifacts/checks/phpantom-120-active.json`.

Full engine source gates passed:18,368 tests /17ignored (excluding the separate
bootstrap child), Clippyfix/fmt/all-target Clippy and clean-cache fingerprint check.
0.1.33 workspace passed1,305 /34ignored, Clippy and formatting. The first native
build failed with ENOSPC while producing an unused static library archive; its log
is retained. Only duplicate non-executable libopenidea_lib.a build outputs were
removed after verifying preserved0.1.32 signature; source, signed apps, executables,
fixtures and earlier evaluation receipts are unchanged. Free space was observed
at3.57GiB afterward. Retry explicitly selects the desktop binary through Tauri's
supported runner arguments (`-- --bin openidea`). Native acceptance remains open.

### Accepted local slice: 0.1.33 / engine 120

The desktop retry built successfully. Selecting `--bin openidea` still produced
the static archive; removing its previous generated copy freed the required peak
space. Both failed and successful build logs remain preserved. The signed arm64
development app is `artifacts/batches/0.1.33/OpenStorm.app`; its source/build receipt
is `artifacts/checks/laravel-policy-0.1.33-build-receipt.json`. Installed optimized
0.1.21 and all earlier accepted apps remain unchanged.

Native About confirmed 0.1.33 and its disposable profile. F1 showed literal request
rules and the declaring ancestor; clicking the source link opened line 29, and
CmdB selected the exact field at 29:14. Gate registration opened provider 91:23;
convention `update` opened BakeryPolicy 29:21; registered `publish` opened
PublishingPolicy 15:21. Their documentation identified the correct handlers.
With focus autosave temporarily disabled, an unsaved max:255 -> max:120 rule change
updated the consumer tooltip with disk bytes unchanged. A real status-bar restart
changed the engine PID and preserved max:120. Closing the modified request prompted;
explicit Don't Save restored max:255 documentation. Autosave defaults were restored.
All baseline app PHP files remain byte-identical. Quit, project/four-tab recovery,
settled field documentation, final process cleanup and deep strict signature pass.
Detailed observations: `artifacts/checks/laravel-policy-0.1.33-native-result.json`.

One early relaunch documentation request returned no result, then a later explicit
F1 resolved the expected rules. This is retained as a startup-readiness limitation,
not a full freshness acceptance. Rule-name catalog completion and the broader
validation/auth matrix also remain open. FEAT-035 and FEAT-066 stay partial;
all 69 areas remain visible (67 partial, 2 user-deferred). The completed source,
client/host, frontend and workspace gates above apply to this exact engine/app.
No new push, publishing, migration or replacement of the installed app occurred.

### First-request documentation investigation

Accepted policy/rule slice checkpoint: `6cdcb03` (local only). Runtime source
hashes still match its signed 0.1.33 receipt; only progress/ledger/engine README
changed after the build. A new actual-client regression opens the pinned Demo
and asks for request-field hover before any completion/navigation warmup, then
repeats after restart. Engine 120 returns `None` on the first cycle (828 ms),
reproducing the native empty first response. This test remains red during diagnosis;
resources and the accepted app remain unchanged. A scratch-only instrumented 121
probe is building to distinguish semantic invalidation from genuine missing data.

The diagnostic trace confirms `ContentModified` (-32801) is being converted to
`null` by the shared position handler. At capture, editor inputs and semantic epoch
still match, but an admitted source publication remains active. Opened PHP callers
with dependent Blade work can retain that publication on their background worker.
The client therefore cannot distinguish temporary invalidation from absent docs.
Scratch 121 now preserves this error for hover only; the client retries that explicit
error within one normal 10-second request deadline, with cancellable 50 ms spacing.
It checks the source buffer identity and engine generation before and after each
attempt. Null results and other errors are final. Two new protocol regressions fail
before the client change and pass afterward: successful retry versus genuine empty,
and bounded retries versus typing/cancellation (tab-close coverage added next).
Full mock suite, Clippy and scratch handler checks are running. No candidate engine
is promoted and no new native acceptance is claimed.

The first-hover actual-client regression now passes on first open and both engine
restarts against the scratch 121 executable (816/725/686 ms observed in that run;
not a startup benchmark). All 12 actual-client and 3 host cases pass. The pending
publication handler regression passes, and scratch Clippyfix/fmt/all-target Clippy
and fmt check pass. All 63 client protocol tests passed, including rejecting an old
response across server restart; a focused follow-up also rejects a successful
response after typing. A premature prototype run failed at setup because its binary
was not yet available; that log is retained separately from the green real run.

Patch 0121 changes only the hover error boundary and its handler regression plus
the changelog. Its pinned series SHA-256 is
`72e046472a6affe511f1af18e9b03dbae04f9fa301f0b83a31c645096a219de7`.
Clean acquisition/build is running; bundled resources remain accepted engine 120.
Source 121, final exact-executable tests and native 0.1.34 gates remain open.
State/handles are tracked in `artifacts/checks/phpantom-121-active.json`.

With no app/client compiler running, 826 inactive generated SDK archives were
removed from the compiler cache, keeping eight newest archives per SDK crate.
Signed 0.1.33 was verified before pruning. Source, signed apps, executables,
fixtures, receipts, earlier evaluations and PHP Workbench remain unchanged. Free
space was observed at 2.35 GiB before and 21.72 GiB after; the exact selection is
recorded in `artifacts/checks/sdk-archive-cache-prune-121.json`.

Candidate 0.1.34 now bundles the clean optimized 121 executable, SHA-256
`91164d9684450ebdabc96fe0dc7cf125d929f91e2bdb807af5f01e3aba665554`.
Its archive, licenses, toolchain and source/build receipt are preserved under
`artifacts/engines/0.11.1-openstorm.121/`; 120 provenance is retained as a tracked
candidate. The final seven frontend freshness cases reject typing, replacement,
file moves, closed views and superseded requests without a false missing-doc hint.
Their focused affected run passes 60 checks. Final frontend gates pass 3,011 tests,
type checking, lint and build (inherited build warnings retained).

The default Herd CLI is PHP 8.3 and rejected the inherited property-hook example;
no example was changed to accommodate it. Rechecking with the preserved workspace
PHP 8.4.23 and `-n` passed all 13 playground/Demo syntax checks and the executable
playground assertions. The native fixture's 88 non-Blade PHP files also pass this
explicit CLI. Static analysis retains exactly the three intentional Demo errors
at 731/951. No system PHP configuration or environment file changed. Full final
engine and root workspace/client gates remain running; native 0.1.34 is not yet
accepted.

Final 121 gates pass: 18,369 source tests / 17 ignored (excluding the separate
bootstrap child), all-target Clippy, formatting and cached source fingerprints.
The exact optimized 121 executable passes all 12 actual-client and 3 host cases.
0.1.34 workspace passes 1,308 / 35 ignored, Clippy and formatting. Native build,
shallow local signing, deep strict verification and source/build receipt passed.

Native 0.1.34 About confirmed arm64, the disposable configuration and attribution.
First F1 correctly showed the request rules/source. A very fast F1 during the
status-bar restart exposed a different handover gap: No language server available
while the old instance stops. A settled retry then worked. The signed candidate
is preserved but this batch is not accepted yet. A new deterministic client test
pauses shutdown and reproduces the exact unavailable response. The correction
keeps the request readiness phase pending through an intentional restart, while
old in-flight requests still cancel at their generation boundary. Tests are running.
The next native candidate must have a distinct 0.1.35 version.

An old automation helper also reopened the previous 0.1.33 app. It was quit; a
unique 0.1.34 binding and About confirmed the correct target before documentation
observations. No editor changes occurred in the extra instance; its session
metadata impact was not audited. The automation session will be reset before the
next candidate. Both app/server processes are now gone and all native fixture app
PHP sources remain unchanged. Details: `artifacts/checks/hover-0.1.34-native-result.json`.

### Accepted local slice: 0.1.35 / engine 121

The restart handover regression passes with 64 protocol checks. New requests wait
through intentional shutdown/startup, while a request already bound to the old
instance still cancels. An additional public editor-extension test reproduced a
late documentation popup after Escape. The final editor correction cancels pending
hover without consuming Escape, preserving ordinary key handling; destruction also
cancels the request belonging to that view. Eight frontend freshness cases now pass,
including unchanged/empty results, typing, document replacement, file moves,
closed views, supersession and Escape. The preliminary 0.1.35 build was not frozen
or signed; the final rebuild includes the Escape correction.

Final gates: 3,012 frontend tests, type checking, lint/build; 1,309 workspace tests /
35 ignored, all-target Clippy and formatting; all 12 actual-client and 3 host cases
against the exact archived 121 executable. Engine 121's 18,369 source tests /
17 ignored, Clippy, formatting, source fingerprints and playground checks above
remain applicable: this handover correction changes the client only.

Signed arm64 development app: `artifacts/batches/0.1.35/OpenStorm.app`.
Frozen source/build receipt: `artifacts/checks/hover-0.1.35-build-receipt.json`.
Native details: `artifacts/checks/hover-0.1.35-native-result.json`. About confirmed
0.1.35, Tauri 2.12.4, the disposable configuration and upstream attribution. First
F1, F1 immediately during a real server restart, Escape with no late popup, genuine
empty hover, and typing an unsaved `xname` while a request was pending all produced
the expected result. Closing the edited tab prompted; Cancel retained it, explicit
Don't Save closed it. Disk source stayed unchanged. Reopening restored `name` and
its rules. Autosave defaults were restored; quit/relaunch without a project argument
recovered the project, tab, caret and first documentation request. Final app/server
exit and post-runtime deep strict signature passed. Exact IPC race boundaries are
covered by deterministic tests; native observations establish the visible outcomes.

One 4.08-second settled sample on this Laravel fixture observed native app RSS
206.31 MiB / 0.49% CPU and engine RSS 159.16 MiB / 0.00% CPU. This excludes WebKit,
GPU/network services and total system footprint. First IPC was 3,408 ms with a
fresh fixture profile and 747 ms on relaunch, not UI-ready time. These startup
observations did not control other validation activity or OS caches. Receipt:
`artifacts/checks/hover-0.1.35-runtime-metrics.json`. No parity claim is made.

The extra 0.1.33 instance's profile metadata was audited without logging settings
values: settings.toml is unchanged since October 28. Its one owned fixture entry
was removed from the real recent-project list with a private backup, preserving
all other entries. Window-state metadata was written by that instance; no earlier
baseline exists, so it was not guessed or restored. Receipts are
`extra-instance-real-profile-metadata-audit.json` and `extra-instance-profile-cleanup.json`.
The CUA session was reset, and subsequent runs used distinct exact-app bindings.

All 69 areas stay visible: 67 partial and 2 user-deferred. In particular, the rule
catalog and broader validation/auth matrix remain open; first-hover recovery does
not accept a whole feature area. No new push, publishing, migration or replacement
of installed optimized 0.1.21 occurred. The next dependent Laravel slice can now
proceed from this locally accepted batch.

### Validation rule-name assistance started

Previous goal turn made verified progress: accepted native 0.1.35 and local
checkpoint `a844533`, with clean worktree and runtime hashes matching the signed
receipt. FEAT-035 still lacks rule-name catalog assistance. A thirteenth actual-client
regression is red on archived engine 121: `string|requ` inside FormRequest::rules()
returns an empty completion list. The acceptance test also requires exact replacement,
installed trait-method navigation/documentation, array rules, inline validation and
negative ordinary-array/parameter/regex contexts. No candidate is accepted yet.

Only inactive generated SDK unit/integration test executables were pruned from the
compiler cache, retaining recent copies and every output from 0.1.35 validation.
Application bin targets, signed bundles, source, fixtures, receipts/logs, PHP
Workbench and earlier saved evaluations were preserved. Free space rose from
2.22 GiB to 15.64 GiB. Selection receipts: `unit-compiler-cache-prune-122.json` and
`integration-compiler-cache-prune-122.json` in artifacts/checks. Scratch engine work
is isolated at `artifacts/phpantom-work/122-validation-rule-names`; source pin,
bundled 121 and signed 0.1.35 remain unchanged.

The first engine regression pass found two failures (5/7 passed): short `int` /
`bool` aliases did not navigate, and the shared test cursor helper counted Unicode
scalars rather than UTF-16 units. The engine's emoji replacement range was correct;
the helper now uses UTF-16. A second pass passed those checks and found gaps in
mixed positional/named arguments and an unfinished EOF string (6/8 passed). Both
have focused fixes under verification. The prototype's actual OpenStorm client
path already passed the initial closed-string completion/navigation/documentation
checks; this is development evidence, not native acceptance or engine promotion.

The parser probe confirmed Mago reports `UnclosedLiteralString` and drops the
containing class, so treating the token as an ordinary literal cannot work. EOF
completion now reparses a private buffer with only the open lexical delimiters
closed and uses the existing version-aware class parser/type resolver. Synthetic
bytes are never indexed, saved or included in edits. An unresolved non-null union
branch now refuses suggestions rather than silently accepting only its Request
branch. Probe output/source are retained in artifacts/checks.

Focused engine tests now pass 8/8 (`validation-rules-122-public-tests-6.log`),
including named/positional calls, aliases, Unicode replacement, ambiguous loaded
and unresolved receivers, unsaved trait changes/close restoration, and private
EOF recovery for FormRequest, Request and facade calls. The expanded real-client
regression also passes, including EOF completion and retained incomplete status
(`validation-rules-122-client-probe-3.log`). These used an explicitly archived
diagnostic prototype with opt-level=0; Cargo.toml has been restored byte-for-byte
to its baseline before linting and the eventual optimized rebuild. No prototype
has replaced the bundled engine.

The formatted 13-file engine change is exported as patch 0122 (SHA-256
`ff571197bca88d4319e0dc8e0cee2c78f4a988e84bf7c7bc918eb62e8978d1f4`).
The 122-patch identity is `cc0563320fa7` (full identity in engine/source.json).
Acquisition/replay produced a new verified source tree; full suite, clean-source
Clippy/format and the normal optimized recipe build are running before binary
promotion. A separate Laravel 13.37.0 corpus copies the preserved 120 baseline,
excludes all environment files, and updates only the pinned Demo.php comment.
The 0.1.36 native fixture validates 108 pinned baseline fingerprints and passes
non-Blade PHP syntax checks with the explicit workspace PHP 8.4.23 CLI. Receipt:
`artifacts/checks/validation-0.1.36-fixture.json`. No app 0.1.36 has been built yet.

This slice completes a statically proven built-in catalog, not the entire
validation/auth gate. Runtime rule extensions, variable/computed rule arrays,
parameter field references, custom receiver implementations and remaining policy
contexts still require separate acceptance. Unknown-rule diagnostics have not
been added by this slice; completion retains incomplete status for dynamic rules.

0.1.36 preparation: frontend 3,012 tests, typecheck, lint and production asset
build pass. Existing hook/chunk/import warnings remain; no new frontend behavior
was introduced. The offline lock refresh changes only the 34 workspace package
versions (receipt `validation-0.1.36-lock-verification.json`). Root all-target
Clippy is running. The full engine suite is still compiling its optimized test
executables; no full-suite success or engine promotion is claimed yet.

Two existing OpenStorm process-launch tests create disposable environment files
with fake FROM_FILE/OVERRIDDEN/PYTHONPATH data, beyond the exact PHPantom-suite
approval. An asynchronous request asks to include those two tests under
`env-file-mutation-guard/SKILL.md`'s explicit-scope rule. They are not run while
that answer is pending; other checks continue. The thirteenth real-client test
also verifies unresolved-union refusal and instanceof narrowing, using no disk
source or environment-file writes (`validation-rules-122-client-narrowing.log`).

The clean 122 suite passes: 18,377 suite tests / 17 ignored, plus one separately
reported FIFO-reader child harness (raw log total 18,378). Its interleaved output
is accounted for in `phpantom-122-source-test-counts.json`. Clean-source all-target
Clippy and format pass. The normal recipe build is finishing; the archived
pre-suite optimized executable reports version 122 and its static Laravel analyze
finds exactly the intentional errors at Demo.php 731 (two) and 951 (one). The
final recipe executable will be checked separately before promotion if its digest
differs. Existing rust-objcopy/libLLVM strip warnings remain nonfatal.

The final normal-release recipe executable is separately archived with licenses,
provenance, toolchain and build receipt at artifacts/engines/0.11.1-openstorm.122.
SHA-256: `f1b2a6b06587afd007c9f74508525d54f37b609be49aa78ba0807273069189e2`.
All 13 real-client and 3 host edit cases pass on that exact digest, and the final
Laravel analyze again finds only the three expected Demo.php errors. Resource
promotion and checksum verification pass; engine121 and previous signed apps
remain archived. Root Clippy/format pass, and workspace tests pass 1,307 / 36
ignored with the two permission-dependent tests excluded (not an unfiltered
workspace gate). No app/engine process remains before native setup. The 0.1.36
development app is building; native acceptance is not yet claimed.

### 0.1.36 native finding: declaration-link fallback

Signed app 0.1.36 launched in its isolated profile with system-only PATH and used
its bundled engine (exact app/child paths recorded). F1 on required displays the
installed rule documentation. CmdB lands on validateRequired at 2168:21. Editing
required to requ produces a required completion; accepting preserves parameters
and neighbors. CmdZ/CmdS produces the exact expected disk diff; redo/CmdS restores
the original file byte-for-byte. Quit exits app/server and post-runtime deep strict
signature verifies. Receipt: `validation-0.1.36-native-result.json`.

The candidate is **not accepted**: plain Rule declaration link follows the existing
in-place documentation convention, but its fallback probes subsequent parameters
and displays `$attribute = string` instead of the method or source. Native
restart/relaunch acceptance is held for the corrected app. A public link-handler
regression reproduces this exact wrong-target behavior; two compatibility cases
cover method docs and explicitly targeted parameter links. The minimal client fix
stops guessed targets at the parameter-list boundary while retaining an explicit
link column. Focused red then green: `validation-doc-link-{red,green}.log`, 14 tests
green including the existing popup suite. Engine122 source/binary are unchanged;
0.1.37 will carry the corrected client with a distinct signed bundle/receipt.

0.1.37 validation: all 3,015 frontend tests pass. Two optional-access checks in the
new test harness were corrected after strict typecheck flagged unchecked array
indexes; final typecheck/lint/build and the final 14 focused documentation tests
pass. Full Rust checks are rerunning on the new version; PHPantom122 has unchanged
source and archived executable, so its passed 18,377 source / 13 client / 3 host
checks remain applicable. The next native run reuses only the owned disposable
0.1.36 profile/project (source restored), exercising a synthetic upgrade and
relaunch without touching installed-user state.

### 0.1.37 native checks: link correction passes; restart stall retained

Signed development app 0.1.37 recovers the owned 0.1.36 fixture/profile with no
project argument. Required/max F1 docs are correct. Plain Rule declaration now
opens the installed method line (2168:1) rather than parameter documentation;
Edit Source opens validateMax exactly at 1710:21. The actual completion list
visually shows mac_address, max and max_digits; Down selects max. Accepting the
completion preserves :255 and neighboring rules. CmdZ/CmdS has an exact disk
witness (only max becomes ma); redo/CmdS restores the original file bytes.

Restart is available in the existing status-bar Language Servers control. Action
search for Restart Language Server had no result; the broader discovery gate
remains open. The restart changes the bundled engine PID (46532 -> 47207), and
initial post-restart F1 max succeeds. F1/Escape leaves no late popup in the
observed cancellation cycle.

The candidate is **not accepted**. After restart with the large framework trait
open, two background widened reference searches time out at 30 seconds. Server
CPU continues for minutes; a later foreground F1 produces no docs by 12 seconds.
One 4.0-second sample observes native RSS 203.45 MiB / 1.00% one-core CPU and
engine RSS 163.56 MiB / 19.45% CPU; this is active post-restart work, **not idle**.
Native RSS excludes WebKit/GPU/network services. First IPC is 3,419 ms on the owned
previous profile, not UI-ready time; OS caches and other system activity were
not controlled. Receipt: validation-0.1.37-runtime-metrics.json.

A two-second native engine sample places the main LSP thread in did_open -> Blade
dependency receiver inference -> Laravel model/mixin class resolution ->
wait_for_pending/nanosleep. Source has a five-second pending-owner escape hatch
and per-thread recursion guard after the pending wait. A cross-thread/variant
cycle is a hypothesis, not yet proven: owner/key instrumentation and a real-client
restart regression must identify it before a fix. Sample and logs are retained
under validation-0.1.37-engine-busy*.

Quit exits the stalled app and engine cleanly. A separate no-argument relaunch
recovers both tabs/caret; first F1 max succeeds. Final quit leaves no app/server
process and post-runtime deep strict signature verifies. Source file is restored
byte-for-byte. Native receipt: validation-0.1.37-native-result.json. Apps 0.1.36
and 0.1.37 remain diagnostic candidates; 0.1.35 is the last fully accepted local
app, and installed optimized 0.1.21 remains untouched. Two existing process-env
tests still await explicit scope approval. All 69 areas remain 67 partial /
2 user-deferred. No push or publication occurred.

### Restart-stall reproduction after local checkpoint bd60163

Checkpoint bd60163 preserves the rule catalog/client link fix and the native
failures. Its compiled runtime hashes match signed 0.1.37; only progress/ledger
and engine README changed after building. Worktree was clean at that checkpoint;
subsequent probe tests are separate uncommitted work.

A direct LspManager control stays responsive for three restart cycles (about
153 ms per paired query), but it has no CandidateFn hook and therefore does not
exercise the host's widened-file path. A real Host project supplies the native
hook. With all four visible request-method/class usage queries, engine122 fails
cycle 2: foreground hover returns Cancelled after about ten seconds. The test
shuts the engine down before asserting and leaves source files unchanged.
Engine121 passes three cycles but has two ten-second waits on this broader case;
this does not establish an engine122-specific cause. Logs: laravel-framework-
restart-{121,122}-four-lens-host-probe.log.

A disposable 123-trace source copy (temporary opt-level=0, bounded explicit-path
trace; no production fix) reproduces the failed hover in cycle 0. Its pending-owner
trace captures circular ownership, including BlogPost/PostCollection and Eloquent/
Query Builder dependencies. The minimal two-class cyclic-mixin test retains both
methods but takes 30.0059 seconds, repeatedly hitting the five-second escape
hatch. This is a focused red regression for class-resolution wait cycles, not a
change to declaration inference or a diagnostic suppression.

A scratch wait-graph correction is under test: retain the key each thread waits
for, detect circular pending ownership before sleeping, and remove wait edges
with an RAII guard. A cycle uses the existing independent-resolution/per-thread
cycle-break path; ordinary contention must still share the identical cached Arc.
No source pin, candidate executable or application resource has been promoted
beyond engine122. Source/trace/binaries and red logs are in artifacts/phpantom-work
and artifacts/checks/pending-waits-123*.

The scratch cycle correction passes five focused concurrent tests, including
ordinary first-resolution Arc sharing. Two- and three-owner mixin cycles each
finish in about seven milliseconds with every method retained, versus the
30-second two-owner red run. The actual Host four-lens/restart case passes three
cycles at 425/385/439 ms with foreground docs retained; trace records two cycle
breaks and no pending-owner timeout. Receipts: pending-waits-123-concurrent-green-2
and laravel-framework-restart-123-cycle-host-probe logs.

Temporary owner tracing and opt-level=0 have now been removed; Cargo.toml matches
the pinned parent byte-for-byte. Scratch production lint/format checks are
running before patch export and an optimized reproducible build. Neither the
engine123 source pin nor a new bundle has been promoted yet.

To preserve build space, 430 inactive generated SDK library/integration test
executables were pruned (13,063,033,856 bytes selected). Two newest copies per
target and every 0.1.37 output were retained; bin targets, signed apps, sources,
fixtures, library objects, receipts/logs, Workbench and evaluation artifacts
remain preserved. The SDK compiler cache was idle; the engine uses a separate
target. Detailed selection/free-space receipt: sdk-test-cache-prune-123.json.

### Engine123 optimized-build gate in progress

The cycle correction is now exported as patch 0123, SHA-256
28f21d9e0e10ba0b63e5b5601e21e14f2e4f09eec7b97a79148747a5fe13019d.
Source acquisition replays all 123 patches into a fresh verified tree with series
161aaafc178c30a81dc28ad8b4392b4efd085ebb7fdd32592a58dc572d9c6860.
The app version is prepared as 0.1.38; resource promotion still awaits the final
normal-release executable and its real-client/host checks. Temporary tracing and
the probe build profile are absent from the exported patch.

Frontend checks pass: 3,015 tests across 348 files, type checking, lint (existing
warnings) and production build. Thirteen engine PHP example files pass syntax
checks and assertions with the isolated PHP 8.4.23 executable. A fresh native
fixture excludes environment files and validates 108 pinned baseline files plus
88 non-Blade PHP files. Engine/workspace regressions and optimized engine build
remain in progress; no new app acceptance or release claim is made.

Clean engine123 gates now pass: 18,379 suite tests / 17 ignored, plus the separate
FIFO-reader child harness. All-target release Clippy and formatting pass; the
inherited nonfatal rust-objcopy/libLLVM stripping warning remains in the logs.
Frontend remains 3,015 passing tests; root workspace is 1,307 passed / 38 ignored,
with only the two process-env tests filtered pending exact-scope approval. Root
all-target Clippy and formatting pass. Lock verification confirms exactly the
34 workspace package versions changed from 0.1.37 to 0.1.38, without dependency
changes. Counts and logs are under artifacts/checks/{phpantom-123,validation-0.1.38}*.
The final normal-release recipe binary and native acceptance are still pending.

The final recipe engine123 executable is archived at
artifacts/engines/0.11.1-openstorm.123/phpantom_lsp with SHA-256
0f53940f50625d031a4a2457b74d439f69757203db92949ef49dfd95abdfd28e.
The archive includes licenses, provenance, recipe receipt and Rust/Cargo identity.
All 14 real-client and 4 actual-host cases pass against those exact bytes. The
Host's four-reference/foreground-hover cycle completes in 154/154/152 ms after
initial documentation through three starts; files remain unchanged. The final
analyzer reports exactly the expected Demo.php errors (two at 731, one at 951).
Source acquisition fingerprints reverify after build.

Engine123 is now promoted to resources and checksum verification passes. A unique
0.1.38 development bundle is building. Native acceptance remains pending; source
tests and mock capability flags do not establish it. All 69 statuses remain
67 partial / 2 user-deferred. No push, publication, installed-app replacement or
update-feed activation occurred.

### 0.1.38 native rule-name/restart slice accepted

Signed development app artifacts/batches/0.1.38/OpenStorm.app runs with a fresh
owned profile and system-only PATH; its engine is the exact archived 123 binary.
About confirms 0.1.38 debug, macOS aarch64, isolated configuration path and upstream
attribution. F1 required/max docs are correct. Plain declaration opens required
source at 2168:1; Edit Source max opens 1710:21. The visual completion list offers
mac_address/max/max_digits, Down selects max, and acceptance retains :255. Saved
undo changes only max to ma; saved redo restores every original byte.

With ValidatesAttributes.php open, the existing status control restarts engine
PID 99490 -> 99831. Initial F1 max arrives and later F1 remains responsive after
usage refresh and caller navigation. The rules usage list contains three expected
assertions callers; selecting the first opens assertions.php at 654:63. The
observed F1/Escape cycle leaves no late popup. Neither first-launch nor relaunch
log contains a reference timeout. This corrects the observed 0.1.37 stall; its
failed bundle, logs and red reproduction remain preserved.

One four-second settled post-restart sample observes native RSS 200.08 MiB /
2.45% one-core CPU and engine RSS 194.88 MiB / 0.00%. Fresh-profile first IPC is
4,946 ms, not UI-ready time. These are single observations with uncontrolled
OS caches/other activity; native RSS excludes WebKit/GPU/network services. They
do not establish controlled fresh/warm performance or parity. Receipt:
validation-0.1.38-runtime-metrics.json.

Quit exits both processes. No-argument relaunch recovers project, two tabs and
caret 29:41; the first F1 max succeeds. Final quit leaves neither app nor server,
request bytes are exact, and post-runtime deep strict signature verifies. Native
receipt: validation-0.1.38-native-result.json. This native slice is accepted; the
full unfiltered workspace/release gate remains open because two existing process
tests need their separate synthetic-env scope approval. Broader validation/auth,
controlled performance and all 69 area gates remain partial/deferred as tracked.
Installed optimized 0.1.21 is untouched. Local only; no push or publication.

### Independent follow-up: discoverable language-server recovery

The rule-name/restart slice is checkpointed locally at 8aefe1a; runtime hashes
match its signed 0.1.38 receipt, with only docs/ledger/engine README changed after
build. The full unfiltered workspace gate still awaits the two separately scoped
process-env checks. Independent action discovery work continues while that
permission is pending.

Native testing found no Restart Language Server action in search; restart was
accessible only through the status popup. A focused registry test fails against
that baseline. The correction registers code.restartLanguageServers as Restart
Language Servers in Code, available for an open project and searchable using
PHPantom/PHP/Laravel/LSP/reload/recovery keywords. It adds no default shortcut.
The handler reads the backend list, ignores a late lookup after project replacement
(including the same root reopened), and waits for all restarts while reporting
individual failures. The existing environment-reload notification shares that
handler; it does not modify environment files.

Focused checks pass for registration/enablement/disposal, current backend versus
stale status state, partial failure and waiting, project replacement, lookup
failure and closed/opening projects. Version 0.1.39 full checks and a new owned
native fixture are in progress. Native discoverability/execution remain pending.

The restart action's final frontend gates pass: 3,023 tests / 349 files, type
checking, lint (existing warnings), and production build. The initial full run
caught the new description's missing Chinese catalog entry; that failure is
retained as restart-action-0.1.39-frontend-tests-red-locale.log and the catalog
is corrected. Notification recovery delegates through the shared action registry,
matching feature boundaries; the focused tests exercise this actual delegation
with an open-project context. Eight action regressions protect current-project
lookup, disposal/enablement, late responses and per-server failure isolation.
The engine123 binary and client/host paths are unchanged from the accepted 0.1.38
slice. Root workspace checks and a unique 0.1.39 development bundle are running.

### 0.1.39 native recovery-action slice accepted

The signed development app artifacts/batches/0.1.39/OpenStorm.app launches the
same checksum-pinned engine123 with system-only PATH and a fresh owned profile.
CmdShiftA finds Restart Language Servers, its description and the shortcut-
assignment hint. Executing that result changes PHPantom PID 8754 -> 9581; the
first F1 max response arrives and later docs remain correct. Searching PHPantom
also finds the command. Existing Settings > Keymap > Search actions lists it;
selecting the row enables Add Keyboard Shortcut. Cancel leaves the keymap intact.

Normal quit exits app/server. No-argument relaunch recovers the owned project,
request tab and caret 29:41, and the first F1 max is correct. About confirms
0.1.39 debug/macOS aarch64, the owned profile and upstream attribution. Final quit
leaves neither process, the fixture is byte-identical, and the post-runtime deep
strict signature verifies. Neither native log contains a timeout. Native receipt:
restart-action-0.1.39-native-result.json. This is functional acceptance, without
a new performance claim; workspace tests were running separately.

Current-project replacement and partial-failure boundaries pass focused source
tests; native partial-failure notifications and real remote recovery remain
unexercised. Full workspace release acceptance still excludes the two separately
scoped environment-file tests. The installed optimized app and all previous
bundles/fixtures remain preserved. No push or publication occurred.

Final 0.1.39 root gates pass: 1,307 workspace tests / 38 ignored / two explicitly
filtered process-env cases, all-target Clippy and formatting. Frontend is 3,023
passed with typecheck/lint/build; existing lint/chunk warnings remain. Lock check
permits only the 34 workspace version changes from 0.1.38 to 0.1.39. Bundle receipt
hashes match every runtime source; only this progress file and feature ledger
changed after build. Native/source receipts are under
artifacts/checks/restart-action-0.1.39-*. Full unfiltered release acceptance stays
open for the pending exact-scope synthetic-env tests, and all 69 statuses remain
67 partial / 2 user-deferred. This local checkpoint does not complete the roadmap.

### Laravel route usages/refactoring acceptance in progress

Continuation after 9b508bc is progress: the 0.1.38/0.1.39 native slices are complete,
and the next Laravel gate is FEAT-030 semantic route usages/rename through the
actual OpenStorm host and guarded editor edit path. The original Workbench matrix
requirement is retained, without adopting its acceptance claims. Static schema
assistance (FEAT-036) and factory workflows remain visible and unaccepted.

A reproducible owned route fixture wraps the existing pinned Laravel acquisition
and syntax/autoload validation. It adds grouped route names, PHP/Blade callers,
Unicode on a caller line, a same-text non-route literal, colliding and duplicate
names, and generated resource routes. No framework boot, database, Composer or
Artisan command is run; environment files are excluded. Script:
scripts/create-openstorm-route-fixture.py; initial receipt:
artifacts/checks/route-refactor-probe-fixture.json.

The first actual-host test finds expected PHP/Blade references and a prepare-rename
range, but the later checked rename returns Cancelled before a guarded preview.
This is a red integration result, not accepted route refactoring. A diagnostic
client build temporarily records only checked-rename error responses to identify
the protocol boundary; this logging must be removed before delivery. Collision
checks are being strengthened to require the specific collision error rather than
treating arbitrary cancellation as a successful refusal. The owned test files are
restored and the engine is shut down before reporting failure. Initial log:
route-refactor-123-host-probe.log.

The diagnostic run proves PHPantom returns -32801 with "PHP project state changed
during the request" while project inputs are being published. It is not a route
collision refusal. The client now keeps one checked-rename deadline, exact source
receipts/target and transient holds across retries of this explicit protocol error.
It stops when any open overlay receipt, primary attachment or engine generation
changes; normal errors/null results are final. Exhausting the publication retry
reports an actionable indexing/retry message. Capability advertisement now lists
the checked-rename extension among explicitly retried methods.

Two focused protocol regressions fail on the prior client; all three pass with
the correction (the restart-invalidation case already passed before it).
They cover identical request receipts and one transient open/close across retry,
a single 30-second virtual deadline, typing/dependency changes/close/replacement,
genuine null/server errors and server restart. Temporary RPC trace logging has
been removed. The strengthened real-host route scenario is rerunning; native
acceptance is pending.

To retain build room, 136 inactive generated SDK library/integration test
executables (3,599,903,056 nominal bytes) were pruned with a detailed receipt,
retaining the newest two per target and every 0.1.38/0.1.39 output. All binary
targets, symlinks, sources, libraries, app bundles, fixtures, engine archives,
Workbench and evaluation deliverables remain preserved. APFS reclaimed space
after a delay; sdk-test-cache-prune-route-refactor.json records observations.

The strengthened real-host case passes after the retry correction: semantic
collision/duplicate/generated/prefix refusals, guarded four-file rename, an
unsaved Blade overlay with two callers, and rejection of a stale preview after
an external caller edit. Explicit reviewed refactoring persists both open and
closed targets by the existing shared transaction contract; the first test's
assumption that an open consumer remained unsaved was corrected against that
contract. Both buffers and exact persisted content are now checked. All owned
fixture files are restored after the case.

Preserved native 0.1.39 exposes a second issue at a grouped declaration: ShiftF6
and its existing dialog show only invoice, discarding PHPantom's full semantic
placeholder openstorm.billing.invoice. Submitting details produces no preview
in the observed cycle; all fixture bytes remain original. Source confirms the
client throws away RangeWithPlaceholder.placeholder. Baseline app/server exit
and post-runtime signature pass. Receipt: route-refactor-baseline-native-result.

A typed RenamePreparation and prepare_rename_details path now retain the semantic
name plus UTF-16 source range. The range-only API/command remains compatible. The
editor uses the existing rename dialog when the semantic placeholder differs from
source text, retaining normal in-place rename otherwise; late preparation is
ignored after edits, document replacement or read-only changes. The native issue
and focused protocol/UI regressions are red before the correction; the initial
corrected run passes 69 protocol and 53 related frontend checks. Updated shared
host/Tauri dispatch and generated bindings are being validated for 0.1.40.
Native acceptance of the corrected bundle remains pending.

Frozen Rust 0.1.40 gates pass: 1,312 workspace tests / 39 ignored, with exactly
the two process-env cases filtered pending their separate scope approval.
All-target Clippy, formatting and binding export pass. All 14 live client cases
and all 5 actual-host cases pass against archived engine123. The host route case
also verifies the renamed grouped declaration's full semantic preparation name
after closed-file edits. A single global cross-file undo transaction remains a
known unaccepted shared-edit limitation from the first batch; native route tests
must record its actual behavior and must not claim global undo parity.

Two additional public frontend regressions expose stale dialog submission and a
late edit-plan response after source changes. Both are now refused, while the
normal in-place -> second ShiftF6 dialog flow remains valid after source restoration.
The final 55 focused editor checks pass. The complete final frontend pass and
refreshed native bundle are rebuilding after this last guard correction.
Earlier unpromoted 0.1.40 bundle output is not the final artifact.

A further 71 inactive generated SDK test executables (1,931,886,128 bytes) were
pruned with the compiler idle, preserving newest-two copies and all 0.1.39/0.1.40
outputs. Apps/source/libraries/fixtures/engine archives/Workbench/evaluation
deliverables remain untouched. Receipt: sdk-test-cache-prune-route-040.json.

Final 0.1.40 frontend gates pass: 3,031 tests, type checking, lint and build.
The final arm64 development bundle is locally signed at
`artifacts/batches/0.1.40/OpenStorm.app`; engine123 is unchanged. This is a
**diagnostic build, not an accepted route-refactoring batch**.

Native checks pass full semantic grouped-route naming, collision refusal,
four-file/five-occurrence preview including two unsaved Blade callers, preview
cancellation, external-change stale refusal without partial writes, and fresh
apply preserving the external comment, Unicode, parameters and unrelated
literal. Blade definition and four PHP/Blade usage results navigate correctly.
Settled navigation after server restart passes; the first immediate request is
not accepted. Native error-hint visibility and fresh startup performance were
not established. CmdZ restored only the active route declaration, leaving the
callers renamed; CmdShiftZ returned all targets to the applied state. The
shared global undo gap is now a reproduced release blocker for this batch.

An early UI binding before controlled app registration launched a second
normal-profile instance. Only read-only navigation occurred; that instance was
quit, inventory checked, and subsequent mutations used the disposable profile.
Its startup timing is excluded. After normal quit, the four owned fixture files
were restored. Relaunch without a project argument recovered the saved project,
tabs and caret; semantic rename preparation and Blade text reflected restored
disk contents. Final quit exited app and engine, and strict signature and all
four original fixture hashes pass. Native receipt:
`artifacts/checks/route-refactor-0.1.40-native-result.json`.

Next required work stays in this batch: shared guarded cross-file undo/redo,
including closed targets, unsaved pre-refactor buffers, later external changes,
and normal editor history ordering. No subsequent dependent Laravel generation
batch starts before this acceptance gap is resolved. The 69-area ledger remains
67 partial / 2 user-deferred. Installed optimized 0.1.21 and previous accepted
0.1.39 recovery-action slice remain preserved.

0.1.41 work in progress addresses the reproduced shared Undo gap. Reviewed
transactions retain bounded original disk bytes and buffer text, including
unsaved originals. Undo/redo validates every target's current disk and overlay,
encoding, line ending, writability, aliases, project roots and history ordering
before the first write. The existing staged recovery/rollback path is shared.
Closed unsaved originals require reopening their target before restoration;
history is session-local and bounded to 16 groups / 64 MiB, independently of
Local History. An unavailable group refuses rather than performing partial undo.

The editor carries a typed shared-history receipt through document sync. Each
open tab/split and attached diff receives the history marker; ordinary typing
retains its existing ordering. Existing key bindings and native menu history
input call the shared restore at the marker, retaining text/history until the
backend transaction succeeds. Source regressions now cover four-file-style
closed/unsaved restoration, later disk/overlay edits, out-of-order transactions,
reopening an unsaved original, split-pane undo/redo and failed-command retry.
Packaged acceptance remains pending; 0.1.40 remains a diagnostic artifact.

The first broad 0.1.41 frontend run found one missing Chinese notification
translation; its targeted catalog gate passes after adding the entry. All 3,036
frontend tests then passed. A later review also wrapped selection-history
commands so they cannot bypass shared undo; its focused 30 checks pass and the
final frontend gates are rerunning. The workspace attempt hit disk exhaustion
while linking the native static library (not a test assertion failure). With
compilers idle, 94 inactive generated SDK library/integration test executables
(3,203,204,184 nominal bytes) were pruned, retaining the newest one per target.
Binary targets/libraries/apps/source/fixtures/engine archives and Workbench /
evaluation deliverables remain preserved. Receipt:
`artifacts/checks/sdk-test-cache-prune-reviewed-041.json`. Verification retry is
in progress; no 0.1.41 app has yet been promoted.

The frozen 0.1.41 source gates pass: 3,036 frontend tests / 351 files, type
checking, lint, production frontend build, Rust formatting and all-target Clippy
with warnings denied. Workspace retry: 1,316 passed / 39 ignored / the same two
process-environment fixture cases filtered pending their separate approval.
All five live actual-host PHPantom cases pass. The route case now verifies
four-target undo/redo, restoration of original unsaved Blade content and dirty
state, and all-or-nothing refusal after a subsequent external caller change.

Before the next static-library link, the owned verification launcher was paused
while its active host tests finished. With all compilers idle after workspace
completion, 126 completed SDK library/integration test executables
(2,737,005,912 bytes) were removed, retaining the newest actual-host executable.
All test results/receipts remain; target binary outputs, libraries, apps,
fixtures, source, archives and Workbench/evaluation deliverables stay preserved.
Receipt: sdk-test-cache-prune-reviewed-041-after-tests.json. The launcher was
resumed for binding verification; the packaged development build is next.

Binding export and post-export type checking pass. The first native package link
again exhausted disk space. Additional test-cache pruning could not supply its
peak space requirement, so the idle, regenerable current SDK static archive
`debug/deps/libopenidea_lib.a` (2,124,015,376 bytes; recorded hash) was removed for
replacement. This supersedes the earlier blanket library-cache preservation
choice only for that intermediate static archive. No rlib inputs, loadable
libraries, app bundles, engine binaries, sources, fixtures, archive provenance,
Workbench or earlier evaluation delivery artifacts were removed. Receipt:
reviewed-history-041-static-link-cache-receipt.json. The native link is retrying;
no packaged acceptance claim is made from successful source tests alone.

Signed 0.1.41 native checks reproduce and correct the original keyboard undo
failure: one CmdZ restores all four disk originals plus the original two unsaved
Blade invoice callers (parameter 9, Unicode retained, modified tab visible).
CmdShiftZ from the Blade tab redoes all four targets. Ordinary typing undoes
first; the next CmdZ undoes the shared refactoring. An external caller change
refuses Undo with a visible actionable notification and no target writes. Removing
only the owned test comment permits retry. Stale preview refusal, exact apply,
post-redo Blade definition navigation (route 5:93), and full semantic preparation
name also pass.

**0.1.41 remains a diagnostic build / batch not accepted.** Native Edit -> Undo
produced no change while keyboard history worked. Source review finds macOS uses
OS predefined history roles, whereas Windows/Linux `appActions.editCommand`
still calls raw CodeMirror undo/redo and can bypass the shared marker. Fix both
menu paths next, preserving text-field behavior, shortcuts and clipboard roles;
add public action/menu regressions and repeat packaged native menu/keyboard
checks before advancing dependent Laravel generated-edit work. A synthetic
beforeinput unit pass did not establish the actual macOS menu behavior.

The native profile's temporary switch-app autosave setting was restored. Normal
quit exited app and engine; the four owned files were restored, then relaunch
without a project argument recovered two tabs and the restored invoice identity.
At the explicit declaration position, rename preparation supplies the full
invoice name; Blade shows the original single caller / parameter 7. Final quit,
strict signature and exact original fixture hashes pass. Installed optimized
0.1.21 remains preserved. Runtime source hashes match the signed bundle.
Receipts: `reviewed-history-0.1.41-build-receipt.json` and
`reviewed-history-0.1.41-native-result.json`. History is session-local, bounded,
and not accepted as persistent global history. No new performance parity claim
or blanket 69-area acceptance is made; ledger remains 67 partial / 2 deferred.

0.1.42 menu correction in progress. A public editCommand regression reproduces
partial local history mutation on the prior action adapter; it is green with
shared reviewedUndo/reviewedRedo. Ordinary programmatic editor edits and focused
text-field browser delegation remain covered. The macOS menu spec regression is
red on the prior OS-role entries. Undo/Redo now route through the existing edit
menu actions on every OS; macOS clipboard, Select All, Services and other native
roles remain intact. No native history accelerator is added, preserving the
existing editor/input shortcut dispatch. All 19 menu specification checks and
22 focused frontend checks pass. Actual packaged input/editor menu acceptance
remains required; the 0.1.41 diagnostic app is preserved.

With compilers idle, obsolete regenerable SDK rlib/rmeta compiler caches were
pruned, retaining newest two per workspace SDK crate/extension. This extends
cache maintenance only to compiler inputs that Cargo can regenerate; external
dependencies, loadable libraries, app packages, source, fixtures, provenance
archives, Workbench and earlier evaluation delivery artifacts are excluded.
Receipt: menu-history-obsolete-sdk-compiler-cache.json. The 0.1.42 version bump
and full affected gates are next; no publishing or new push is authorized here.

Source review caught a modal-window consequence before packaging: native Settings
blocks ordinary project menu actions and does not activate the project menu
bridge. Routing its Undo through the parent proxy would undo a background editor.
A focused Settings listener now handles only local history through the same edit
adapter, without project action execution or project menu reconfiguration. Two
new regressions are red before the listener; all 31 focused Settings/action/menu
checks pass after it. Backend modal routing still needs its targeted correction
and gate. Main editor/action tests alone do not prove Settings field behavior;
actual native text-field checks remain required.

The obsolete compiler-cache receipt records 4,314 files / 23,327,731,712 bytes
pruned with compilers idle. Durable app/engine/source/provenance deliverables
remain untouched. This restored enough room for reproducible later builds;
temporary Rust compiler caches are distinct from the preserved delivery artifacts.

The initial workspace run exposed an inherited JDWP relay-test stall: the test
released its selected port before rebinding, then retried the client forever
with no live listener. The confirmed live stalled SDK test process was stopped;
that workspace attempt is a failure, not acceptance. Receipt:
menu-history-workspace-jdwp-stall.json. The existing listener acceptance body is
now shared privately with the test, which owns its actual bound listener and
has a 10-second handshake deadline. Public debugger behavior and features remain
unchanged; FEAT-047/048 stay user-deferred. Both focused JDWP cases pass, and the
relay case passes 20 repeated runs. The final workspace gate is rerunning.

The backend Settings menu-routing regression also reproduces the missing path
and is green after explicit routing: Undo/Redo stay in the Settings window,
existing app/settings actions go to its parent, other project actions remain
blocked. All 48 native-unit cases pass, including all 19 menu specification
cases. Final frontend and packaged Settings/editor menu evidence are pending.

The first final workspace retry passed JDWP, then failed the process-test helper's
unconditional `info.running` assertion while starting `true`. Source shows the
waiter/pump runs before `start()` returns its snapshot, so an already-completed
command is valid. The test helper now accepts that race; its scenarios still
assert output/exit events, and the pruning case explicitly checks its held
second process is running with a PID. No production process-launch behavior is
changed. The two environment-file cases remain excluded under their separately
pending scope; no environment files were changed for this correction.

Final frozen 0.1.42 source gates pass: 3,040 frontend tests / 352 files, type
checking, lint/build, Rust workspace 1,318 passed / 39 ignored / the same two
pending environment-fixture cases filtered, formatting, all-target Clippy with
warnings denied, five live actual-host cases and binding export. The signed
arm64 development app is preserved at `artifacts/batches/0.1.42/OpenStorm.app`.

Native menu history now works: Settings menu Undo/Redo/Undo changes only its
query while all four project files remain renamed; project Edit -> Undo restores
all four disk originals plus the original two unsaved Blade invoice callers /
parameter 9 / modified state, and menu Redo from the Blade tab reapplies all
four targets. A subsequent external caller edit refuses menu Undo with a visible
warning and no writes. Editor CmdZ/CmdShiftZ still works, and Blade definition
after these cycles navigates to route 5:93. Preview cancellation and exact apply
also pass, preserving Unicode, parameters and an unrelated same-name literal.

**0.1.42 remains diagnostic / batch not accepted.** The native field check found
that Settings CmdZ has no effect after the native undo role was replaced, while
menu Undo still clears the query. Restore field keyboard compatibility without
intercepting editor keymaps, IME or already-handled events; repeat main/Settings
field and editor checks in the next candidate before advancing Laravel work.
The field failure receipt is menu-history-0.1.42-field-shortcut-failure.json.

The disposable profile's switch-app autosave default was restored. Normal quit
exited app and engine, all four owned files were restored, and strict signature
passes. No relaunch/performance acceptance is claimed for this diagnostic
candidate. Runtime source hashes match its signed receipt. The combined native
receipt is `menu-history-0.1.42-native-result.json`; all 69 areas remain visible
at 67 partial / 2 user-deferred.

0.1.43 field-key compatibility correction in progress. The native 0.1.42 failure
is protected by failing plain-field CmdZ/CmdShiftZ regressions. Browser guards,
already installed in both workbench and secondary Settings windows, now supply
native macOS field history through browser edit commands. Editor-owned targets,
IME/composition-ending keys, already-handled events and other modifier shortcuts
stay outside that path. The logical shortcut key is honored independently of
physical keyboard layout; a further regression caught the page-key guard
interfering with that case and is corrected. The normal browser path is retained.
The workaround is desktop macOS only and adds no settings interface or keymap
binding. Focused source gates and a new uniquely versioned package are required;
no native field-key acceptance is inferred from mocks.

0.1.43 field-key correction accepted in the signed arm64 development app at
`artifacts/batches/0.1.43/OpenStorm.app`. Native Settings query CmdZ/CmdShiftZ
passes with explicit focus, including after menu Undo/Redo. While the reviewed
route refactoring remains undoable, those field commands change only the query:
all four persisted project targets stay applied. Main Go to Line also restores
5:93 / 11:1 through keyboard Undo/Redo and Cancel retains the editor caret.

The packaged route corpus passes preview Cancel, fresh five-occurrence/four-file
apply with two unsaved Blade callers, exact Unicode/parameter preservation and
an unchanged unrelated same-name literal. Native menu Undo restores all four
disk originals plus the two-call unsaved Blade overlay / parameter 9 / modified
indicator. Redo from the Blade tab reapplies all targets. A later external caller
edit visibly refuses menu Undo without changing any target. Removing only the
suite-owned external comment permits editor CmdZ/CmdShiftZ to restore/reapply
all four files. Blade CmdB reaches route 5:93 after these cycles; ShiftF6 sees
the full renamed semantic key and was cancelled.

The disposable profile's original switch-app autosave true / idle autosave false
was restored. Normal quit exited the app and engine. After restoring only the
four owned fixture files, no-project-argument relaunch reopened the project and
two tabs with original invoice declaration and Blade one-call / parameter 7
content. Second normal quit also cleaned up both processes. Strict bundle
signature passes. Installed optimized 0.1.21 remains unchanged (binary SHA-256
048064d8cc6beb3985f36a445a5524992624e0413c919c1faa61cbd27d49fa16).

Final source gates: 3,044 frontend tests / 352 files, type checking, lint, frontend
build, 1,318 workspace tests / 39 ignored / two separately pending SDK env cases
filtered, formatting, all-target Clippy with warnings denied, five live host
PHPantom integration cases, binding export and native app build pass. The two
SDK env cases still await their distinct mutation-scope answer; the unfiltered
root gate is not claimed. Engine .123 is unchanged. Source-key exclusions cover
IME/editor/already-handled events; other-OS native and IME acceptance remain open.
Evidence: `field-history-0.1.43-native-result.json`, build receipt, workspace
counts and per-phase exact disk witnesses under artifacts/checks. The .41/.42
apps remain preserved diagnostic candidates; .43 resolves their history blockers.
No installed-app replacement, publishing or new push. All 69 areas remain
visible, 67 partial / two user-deferred. Next: continue FEAT-030 Laravel route
parameter/controller/registration and duplicate-diagnostic corpus, preserving
static-only inspection and guarded edits.
