# OpenStorm implementation and acceptance

Base: thinkycz/openidea 50212e73972d6f05bf16050cb0ba10102a6c8054 (0.1.21).
Branch: codex/openstorm-php-laravel. Target: macOS Apple Silicon, local delivery.

Resumed delivery cadence (2026-11-11): finish one independently verifiable slice,
update this record, commit, push, verify the remote ref and clean worktree, then
start the next slice. Reuse unchanged acceptance evidence; retain every unrun
native/platform case and whole-area status. Do not combine unrelated feature
work into a large unpushed batch.

Build retention slice (2026-11-11, verified local cleanup): user now requires
small committed/pushed slices and only the latest generated build. Run
`python3 scripts/prune-openstorm-builds.py` to review, then add `--apply` after
each successful package. It retains the numerically newest version in
`artifacts/batches`, checks every bundle identity/version before deleting any,
refuses symlinked archive paths, and removes only obsolete `.app` directories.
Receipts beside bundles remain. Four Python regressions cover numeric ordering,
actual deletion with receipt/latest preservation, foreign identities, symlinks,
and empty inventories. These pass; a second dry run reports no obsolete bundles.

Local cleanup removed 31 older archived apps (0.1.22–0.1.52), leaving
0.1.53. Also removed two duplicate 0.1.22 candidates, the pre-conflict app copy,
superseded archived engine executables, and the disposable PHPantom compiler
cache (6,240,402,992 logical bytes in those additional paths). Sources, patches,
licenses, provenance, regression receipts, fixtures, engine130 and the original
released114 evaluation baseline remain. Installed OpenStorm was untouched.
Evidence: `artifacts/checks/build-retention-{dry-run,applied,extra}.json`.
A follow-up inventory removed15 obsolete Mach-O engine probe executables
(1,745,474,640 logical bytes), after checking executable headers, inactive
process paths and frozen metadata; SHA-256 records are in
`artifacts/checks/build-retention-old-probes.json`. Their source snapshots and
baseline evidence remain. No native feature acceptance is claimed by cleanup.

Native slice54 infrastructure recovery: frontend gates passed; the prior workspace run
exited101 after its external Cargo target directory disappeared during linking.
The old external Rust toolchain and temporary launcher environment are now also
absent. This is an infrastructure failure, not a passing Rust/native check.
Restored Rust1.99.1 (including rustfmt/Clippy) and pnpm11.28.9 under ignored
`artifacts/toolchains`, without changing global configuration. The rerun uses
only `artifacts/build` with incremental compilation disabled. The cold workspace
run passes1,738 tests/60ignored/0filtered in1,354.11s; Rust formatting passes.
All-target Clippy with warnings denied passes in358.25s. The interrupted receipt is not acceptance.
`artifacts/checks/slice54-workspace-totals.json` records the totals; the
post-bindings frontend type check and all8 packaging/retention tests also pass.
The temporary fixture also expired; it has been recreated under
a fresh `/private/tmp/openstorm-slice54-*` directory with PHP syntax/runtime
and PSR-4 mapping validated. It is outside the repository ancestor ignore rules
respected by PHPantom workspace walkers.

Slice54 — merged-source native package (2026-11-11, verified): OpenStorm0.1.54
with unchanged PHPantom130, built/signed in253.16s. The canonical local bundle is
`artifacts/batches/0.1.54/OpenStorm.app` (arm64, ad hoc, not notarized). The new
`python3 scripts/package-openstorm.py` recipe validates identity/architecture,
signs and records engine/source provenance, then prunes older app archives and
cleans the owned `artifacts/build` output. It refuses symlinked archive/build
roots, nonnumeric versions, existing receipts and shared/evaluation targets.
All8 packaging/retention tests pass. Packaging removed21,499 compiler files
(18.6GiB reported by Cargo); only the latest archived app remains. No app was
installed or published. Installed0.1.21's executable still matches its preceding
receipt (`048064d8…9fa16`).

Native observations used an explicit disposable HOME and runtime identifier
`cz.thinky.openstorm.dev.w8f8a5ceba98300ce`; shell environment import was disabled.
The fixture passed PHP syntax/runtime and Composer PSR-4 mapping checks. Search
Everywhere and the existing menus/layout opened main.php; the bundled engine
ran from this exact app's Resources with `--stdio --no-project-tools`.
Declaration navigation opened a readable `phpantom-stub://DateTimeImmutable`
document labelled Library source (read-only); typing x left its text unchanged.
Cmd+B on total resolved to src/Invoice.php line6. Native Basic completion
restored the method name after deletion; Cmd+S persisted exactly the original
caller with no diagnostics. Paste automation was unreliable, so exact partial
prefix ranking is not accepted by this smoke test.

Native server Restart replaced PID54421 with54795; the old PID exited and Cmd+B
worked afterwards. A real dirty local conflict.txt plus an external disk witness
caused Cmd+S to ask before overwrite. Cancel retained both versions and an
Autosave paused notice. Close Project > Don't Save discarded only local text,
preserved external bytes and stopped the server. Reopening restored3 clean file
tabs with the external contents; the discarded edit did not return. The builtin
stub tab was not restored and is recorded as a recovery limitation. All4 fixture
originals were restored. Native Quit stopped app/server processes; no-argument
relaunch restored the project,3 file tabs and main.php caret5:16, with PHPantom
Running. Final native Quit again left no owned app/server processes. Deep/strict
signature verification and executable hash still passed after runtime. Every
source hash matched the build receipt before this documentation-only closeout.

Evidence: `artifacts/checks/slice54-{frontend,sdk-gates,workspace-totals,
app-build,build-receipt,native-progress,installed-readback}.json`, the matching
logs, and `artifacts/batches/0.1.54/{build-receipt,retention}.json`. Physical
user-controlled Dock-switch autosave remains unverified; broader platform,
service and performance cases remain open. Whole-area statuses stay67partial/
2user-deferred. Laravel navigation continues in a separate bounded slice after
this slice's commit/push; no whole-area acceptance is inferred from this smoke.

Slice55 — recover Laravel navigation queries (2026-11-11, verified): failed
route/view list and target lookups now offer an explicit Retry default action.
Query start records the selected logical name/phase and clears previous selectable
rows; retry preserves that name and the incomplete-result warning. A current
indexing cancellation now shows Cancelled/Retry instead of leaving Loading stuck.
Closed/superseded queries remain discarded, and project replacement cancels a
pending retry. No automatic polling, project execution or new settings were added.

The two missing-Retry UI tests fail first, then pass. A separate cancellation
regression fails with Loading stuck, then passes. All9 chooser tests and all5,325
frontend tests pass, plus type checking, lint, Knip, formatting and asset build.
The unfiltered Rust workspace passes1,738 tests/60ignored/0filtered in1,225.75s;
Rust formatting and all-target Clippy with warnings denied (313.67s) pass.
Bindings remain unchanged. Engine130 is unchanged. The signed arm64 development
app0.1.55 builds in211.35s; only this archived app remains. Packaging removes
the duplicate SDK bundle and21,499 compiler files (18.6GiB).

Native acceptance uses the pinned Laravel13.37 fixture: source/autoload metadata
verified,88 non-Blade PHP files linted, environment files excluded, application
never booted. The exact packaged engine runs with `--stdio --no-project-tools`
under a disposable HOME/worktree-specific identity. Controlled SIGSTOP pauses of
only this owned engine produce real30s request timeout errors; Retry succeeds
without closing the chooser for all4 paths: route list, named-route targets,
view list and named-view target. Enter activates the Retry default. The selected
nav.duplicate survives failure, exposes both targets, and clicking the second
opens routes/openstorm_navigation_duplicate.php4:80. The selected openstorm-stock
survives failure and opens resources/views/openstorm-stock.blade.php1:1.

A fifth real timeout followed by Retry -> Loading -> Cancel closes the pending
query. After engine recovery, a fresh route chooser succeeds without a late error.
Every pause helper finishes:3 explicit resumes and2 automatic60s safety resumes;
the latter happened before the later explicit resume calls. Normal Quit exits
app/server. No-argument relaunch restores3 tabs, the selected caller and caret1:1;
fresh route lookup succeeds. Final Quit leaves no owned processes. All12 owned
fixture originals and Composer metadata remain exact. Post-runtime signature and
executable/source hashes match the build receipt before documentation closeout;
installed0.1.21's previous executable hash is unchanged. No compiler output remains.

Evidence: `artifacts/checks/slice55-laravel-{retry,cancellation}-{red,green}.log`,
`slice55-{frontend,sdk-gates,workspace-totals,app-build,build-receipt,
laravel-fixture,native-progress}.json`, per-case `slice55-*-suspend.json`, and
`artifacts/batches/0.1.55/{build-receipt,retention}.json`. Current-indexing
cancellation and project replacement during Retry have source regressions;
native controlled failures used request timeouts. Physical Dock-switch autosave
and broader area/platform/service/performance requirements remain open. Whole-area
statuses stay67partial/2user-deferred; this accepts the bounded recovery slice.

Slice56 — browse another Laravel name without closing (2026-11-11, verified):
Back now returns from route/view targets to a fresh name list in the same chooser.
It is available for ready, empty, failed and still-loading target queries. It
cancels the previous target request; late responses cannot replace the new list.
Existing incomplete warnings persist. The dialog uses its established extra-button
role and platform order; no settings or global keybindings change. Default Enter
remains Show Targets/Open/Retry; Tab/Enter can activate Back explicitly.

Three UI regressions fail before Back exists and pass after: fresh route names,
return after failure, and pending view-target cancellation/late-result rejection.
All12 chooser tests and all5,328 frontend tests pass, plus type checking, lint,
Knip, formatting and asset build. The unfiltered Rust workspace passes1,738 tests/
60ignored/0filtered in1,202.11s; Rust formatting and all-target Clippy with warnings
denied (323.02s) pass. Bindings remain unchanged. Engine130 is unchanged. The
signed arm64 development app0.1.56 builds in223.47s. Packaging keeps only this
archive and removes its duplicate SDK bundle plus21,499 compiler files (18.6GiB).

Native checks use a fresh pinned Laravel13.37 copy:88 non-Blade PHP files linted,
Composer metadata verified, environment files excluded. The exact bundled engine
runs under the disposable HOME with `--stdio --no-project-tools`; no framework
boot runs. Native menu route browsing shows Back alongside Open/Cancel. From the
nav.duplicate targets, Back returns to names without opening a file. Adding a
syntactically checked external fixture route while targets are open, then Back,
finds openstorm.back.fresh and its URI: the names are re-queried. Tab twice focuses
Back; Enter returns to names. Pausing only the owned engine -> target Loading ->
Back -> names Loading -> explicit resume returns names; the late target result
does not replace them.

Removing the owned openstorm-stock view while names are open yields no targets
and disabled Open. Back returns to fresh names that exclude the removed file.
Restoring its exact bytes and reopening finds it again. A second owned-engine
pause produces a real30s view target timeout; Back clears the error, requests
names and succeeds after explicit resume. Enter then shows the restored target,
and a second Enter opens resources/views/openstorm-stock.blade.php1:1. Both
pause helpers finish explicitly before their60s safety watchdogs.

All12 originals match exact bytes, Composer metadata is unchanged and the extra
route is removed. Native Quit exits app/server. No-argument relaunch restores2
tabs and the selected Blade caret1:1; fresh admin.users.index lookup resolves
routes/web.php28:16 and Back returns to names. Final Quit leaves no owned process.
Post-runtime signature/executable/source hashes match the build receipt before
this documentation closeout. Installed0.1.21's prior hash is unchanged, and no
compiler output remains.

Evidence: `artifacts/checks/slice56-laravel-back-{red,green}.log`,
`slice56-{frontend,sdk-gates,workspace-totals,app-build,build-receipt,
laravel-fixture,native-progress}.json`, `slice56-fixture-changes.jsonl`, both
`slice56-*-suspend.json` receipts and `artifacts/batches/0.1.56/{build-receipt,
retention}.json`. This accepts the bounded Back workflow. Physical Dock-switch
autosave and wider69-area/platform/service/performance gates remain open;
whole-area statuses stay67partial/2user-deferred.

Slice57 — PHP builtin freshness (2026-11-11, verified): fixes stale builtin
contents/context after a PHP target change and server restart. The real-client red
case independently witnesses a newly applied PHP8.3 target with an uncached class,
while reopening the original builtin URI still returns8.2. Frontend URI caching
and readonly document reuse independently reproduce stale data too.

PHP builtin opens now fetch fresh contents/context, check advertised target support
and refuse a changed target or server generation during loading. Other immutable
library caching remains intact. The host refreshes already-open, clean readonly
buffers through the existing document-patch channel before returning new metadata.
Refresh adds no references, dirty state, user edit or disk write. Observers receive
close/open snapshots rather than user didChange notifications. No settings,
shortcuts, appearance, save-conflict or debugger behavior changes.

The three original regressions fail before the fixes and pass afterwards; the
mixed-target regression refuses a raced result and permits the next stable one.
The actual pinned-engine client restart test passes, and the actual-host case
passes in6.83s with fresh PHP8.3 metadata and full replacement text in the existing
readonly/clean document. All5,330 frontend tests/type checking/lint/Knip/formatting/
build pass. The sequential unfiltered Rust workspace passes1,740 tests/62ignored/
0filtered, including doctests, in350.58s; Rust formatting, post-bindings frontend
type checking and all-target workspace Clippy with warnings denied (314.99s) pass.
The first workspace attempt passed regular tests but failed an ide-http doctest
with a missing rustls artifact while a separate host build reused its target.
Its failed receipt is retained separately; the successful rerun is sequential.

Signed arm64 development app0.1.57 builds in208.70s with unchanged PHPantom130.
The sole generated archive is `artifacts/batches/0.1.57/OpenStorm.app`; packaging
prunes0.1.56, removes the duplicate SDK bundle and cleans27,493 compiler files
(23.3GiB reported by Cargo). Installed0.1.21's preceding executable hash remains
unchanged. No app installation, publishing or notarization occurs.

Native checks use a disposable HOME, restricted PATH and syntactically checked
plain-PHP/Composer fixture; no environment files or project tools run. Existing
Go to File and Cmd+L/Cmd+B open DateTime with a PHP8.2 readonly banner. External
fixture Composer platform8.3 plus the existing server Restart action replaces
PID12263 with12272. Navigating from the same main.php caret2:8 reuses the exact
`phpantom-stub://DateTime?project_context=1` URI and existing tab, now labelled
PHP8.3. Focused typing x and Save All leave the observed2,048 stub characters
unchanged and the view clean; main.php's original disk bytes are unchanged.

Normal native Quit stops app/server. A no-argument launch restores main.php at2:8;
explicit Cmd+B opens readable readonly PHP8.3 declarations again. The builtin tab
itself is not restored, retaining the existing recovery limitation. Final native
Quit stops app/server again. Both fixture originals are restored byte-for-byte.
Post-runtime deep/strict signature, executable and engine checksums pass; every
build source hash matches before this documentation-only closeout. Only the latest
archive remains, and compiler output is absent.

The separate live Composer8.3->8.4 old-URI polling experiment times out. Its
initial interpretation as requiring restart is corrected by Slice58 below:
Composer changes already rebuild the engine's project context, retiring the old
builtin URI. A cancellation from that URI is not evidence of an unapplied target.
Engine130 is unchanged. Wider version/extension/framework gates, physical
Dock-switch autosave and broader platform/service/performance acceptance remain
open. Whole-area statuses remain67partial/2user-deferred.

Evidence: `artifacts/checks/slice57-{focused,host-builtin,frontend,sdk-gates,
workspace-totals,app-build,build-receipt,native-progress,target-fixture}.json`,
matching red/green logs, `slice57-sdk-first-artifact-failed.json`,
`slice57-target-live-failed.log`, `slice57-fixture-changes.jsonl` and
`artifacts/batches/0.1.57/{build-receipt,retention}.json`. Final commit/remote
read-back and source reconciliation are recorded in the ignored delivery receipt.

Slice58 — live Composer PHP targets (2026-11-11, verified):
corrects Slice57's overbroad restart requirement. Direct protocol observation of
unchanged PHPantom130 shows Composer platform8.3->8.4 is applied immediately and
navigation returns a new builtin project_context. The previous polling test kept
asking the retired URI and swallowed cancellation, then mislabelled its timeout.
No engine or application runtime change is needed for the observed live workflow.

A new actual-client regression navigates from live source for every applied target,
checks one unchanged server PID through8.3->8.4->8.3, refuses retired builtin URIs,
and verifies version-filtered array_find completion with array_filter as a positive
control. Its valid PHP fixture retains an unsaved dependency with a Unicode comment;
precise declaration navigation and unchanged dependency disk bytes survive both
context rebuilds. The focused regression passes in3.84s, contexts1->3->5.

Native checks reuse the signed0.1.57 app whose runtime sources and engine remain
unchanged. A fresh isolated HOME/Composer fixture passes PHP syntax/runtime checks;
no environment files or project commands run. Existing Ctrl+Space at main.php3:9
shows3 candidates under8.3 and5 under8.4, adding array_find/array_find_key. Actual
file-watcher delivery and Cmd+B open readonly DateTime contexts1/3/5 with8.3/8.4/8.3
labels, all in serverPID14393, without server restart. Old context tabs retain
historical snapshots; fresh navigation opens the current target's identity.

Down3 visibly selects array_find; Tab replaces the function name and Save All
persists exactly the expected edit while preserving its arguments. The resulting
file passes PHP8.4.23 syntax/runtime. Native Edit Undo/Redo and File Save All persist
original/completed/original bytes. An initial keyboard Undo attempt left a
Definition popup and did not change the source; menu actions establish acceptance,
without claiming that automation focus/shortcut limitation is repaired. Completion
screenshots expose all rows; accessibility exposes only the first popup item.
After downgrading, the8.4-only rows disappear and fresh navigation shows8.3.

Normal Quit stops app/server; no-argument relaunch restores clean main.php2:8 and
Cmd+B opens readonly8.3 again. Final normal Quit stops app/server. Both original
fixture files are byte-identical. Builtin-tab recovery and physical Dock-switch
autosave remain unverified as before. All5,330 frontend tests/type checking/lint/
Knip/formatting/build pass. The unfiltered Rust workspace passes1,740 tests/63ignored/
0filtered, including doctests, in1,164.39s; the new ignored real-engine case was run
explicitly above. Rust formatting, post-bindings frontend type checking and
all-target Clippy with warnings denied (306.79s) pass.

Post-runtime deep/strict signature, executable/engine checksums and installed0.1.21
read-back pass. Only the new ignored integration test and two acceptance documents
differ from the0.1.57 build receipt; all runtime sources still match that signed
app. Reusing it avoids generating an identical runtime for test/documentation
changes. Cargo cleanup removes21,176 compiler files (16.4GiB reported), leaving no
compiler output and only the latest0.1.57 app archive. All69 IDs remain visible at
67partial/2user-deferred. Final commit/remote read-back and source reconciliation
are recorded in the ignored delivery receipt.

Evidence: `artifacts/checks/slice58-client-live-target.log`,
`slice58-{client-regression,client-fixture-syntax,frontend,sdk-gates,workspace-totals,
app-readback,compiler-cleanup,target-fixture,native-progress,completion-applied,
completion-undo,completion-redo,wire-probe}.json`, `slice58-fixture-changes.jsonl`
and matching logs. The direct wire probe additionally confirms retired-URI
cancellation (-32801) and a clean process exit after the client closes stdin.

Slice59 — stale Laravel navigation selections (2026-11-11, verified): protects
queued row callbacks after Back, chooser replacement or project replacement. Three
regressions reproduce the old handler opening `/p/routes/web.php` after project
`/q` is active, opening a former target after Back, and using an old route name
in a current views query. Late-response generation checks do not protect these
separate selection callbacks.

Selection now requires an item from the current result snapshot, after existing
loading/project checks. Stale names/targets leave the current chooser intact;
fresh selections retain duplicate handling, Back/reselection and normal opening.
All15 chooser checks pass; the3 new cases fail before the correction for the
unintended open/query. All5,333 frontend tests/type checking/lint/Knip/formatting/
build pass. The unfiltered Rust workspace passes1,740 tests/63ignored/0filtered,
including doctests, in1,177.68s. Rust formatting, post-bindings frontend type
checking and all-target Clippy with warnings denied (306.47s) pass.

Signed arm64 development0.1.58 builds in208.99s with unchanged PHPantom130.
The sole generated archive is `artifacts/batches/0.1.58/OpenStorm.app`; packaging
prunes0.1.57, removes its duplicate SDK bundle and cleans21,499 compiler files
(18.6GiB reported by Cargo). No app installation or publishing occurs. Post-runtime
deep/strict signature, executable/engine checksums and installed0.1.21 read-back
pass. Every source hash matches the build receipt before this documentation-only
closeout; compiler output is absent.

Native fixtures are independent pinned Laravel13.37 copies with15 original witness
files each,97 non-vendor/non-Blade PHP files syntax-checked per project, verified
Composer mappings/lock/helper metadata and no environment files. Recent-project
entries/trust were prepared through the baseline57 native UI in a disposable HOME;
acceptance runs the new58 package. Don't ask again remains unchecked. The bundled
engine runs from this app's Resources with --stdio --no-project-tools; no framework
or package commands run.

Project A initially shows only its unique route under openstorm.switch. Pausing
only owned engine41304 -> native Routes shows Loading/disabled Show Targets ->
File > Open Recent B > This Window closes the chooser and opens B without old A
tabs/results. The paused old engine exits; its60s watchdog never fires. Fresh B
query shows only openstorm.switch.b.only; Enter/Enter opens B's route file3:61.
Then pausing owned engine41813 during B's selected view-target lookup shows
Loading/disabled Open. Replacing B with A closes that chooser, restores A's caller
and retires B's paused engine, again before the watchdog. Fresh A views show only
openstorm-switch-a-only. Show Targets > Back > refilter > Enter/Enter opens A's
view1:1 with its distinctive witness. These native cases prove pending-operation
cancellation/process retirement and fresh selection. Successful late replies and
queued stale-row callbacks are covered by frontend regressions, not claimed
observed from a retired native process.

A warmed10.020s two-process sample after B route navigation observes native app
RSS217888KiB at both ends and CPU time6.57->6.87s; engine RSS214016KiB at both ends
and CPU time3.59->3.59s. This excludes WebContent/GPU/other processes and is not total
memory, fresh-startup or parity evidence. Wider performance gates remain open.

Normal Quit stops app/server. No-argument relaunch restores A's2 clean tabs with
its Blade witness selected at1:1 and No problems found. Fresh Routes from that
recovered Blade tab finds only A's witness and opens A's route3:61. Final normal
Quit stops app/server; no paused helper/engine remains. All30 witness originals
and both Composer metadata sets are byte-identical. Physical Dock-switch autosave,
broader route/view-data/project/configuration/cancellation matrices and other
69-area gates remain open; statuses stay67partial/2user-deferred.

Evidence: `artifacts/checks/slice59-stale-row-{red,green}.log`,
`slice59-{stale-row,frontend,sdk-gates,workspace-totals,app-build,build-receipt,
project-a,project-b,profile-preparation,native-progress,warm-idle-sample,
app-readback}.json`, both `slice59-*-suspend.json` receipts and
`artifacts/batches/0.1.58/{build-receipt,retention}.json`. Final commit/remote
read-back and source reconciliation are recorded in the ignored delivery receipt.

Slice60 — select the Composer document for Laravel navigation (2026-11-11,
accepted local development0.1.59): the typed route/view adapter omits the source URI supported by
PHPantom's project router. Default outermost root markers share one server across
a parent Composer project and a nested independent project. With both documents
open, the real-client red case fails with Select a Composer project document.
The protocol regression independently confirms the missing URI field. This is a
separate context-selection defect, not evidence explaining the old46 first
declaration-lookup miss; that readiness claim remains unclosed.

Both requests now include the checked source snapshot's typed URI. Capability
checks, source-revision validation, cancellation, limits and incomplete results
remain intact. Two focused protocol cases pass, including unsupported-server
refusal and duplicate/incomplete preservation. The actual engine case passes
in3.44s in one process through outer/inner/outer: independent names and same-name
route/view destinations stay with the selected document. An unsaved inner route
is excluded from outer results and resolves precisely after a Unicode comment;
all disk originals remain unchanged. The nested folder contains a Unicode name.

Development0.1.59 keeps PHPantom130 unchanged. All5333 frontend tests,
typecheck, lint, knip, format and production build pass. The existing0.1.58
packaged app independently reproduces both Routes and Views failures with the
outer and nested caller tabs open in one window/server. Both choosers report
Select a Composer project document; normal quit exits app47200 and engine47568.
The full Rust gate initially exposed an accidental cmake version replacement in
the app-version bump; restored the original locked0.1.58 dependency/checksum
and verified that only35 local package versions change. The failed receipt is
retained. The corrected full workspace passes1740tests/64ignored/0filtered
in1149.76s; formatting, post-binding typecheck and Clippy306.84s pass. The new
ignored actual-client scenario was explicitly run, not counted as a regular
workspace pass. Exact generated plain-PHP route sources, including the unsaved
Unicode variant, pass PHP8.4.23 -n syntax checks. The development binary emits
the existing large-unwind-table linker warning; no failed source gate remains.

The signed arm64 development app packages in209.52s. Its verified receipt and
strict/deep signature precede removal of obsolete0.1.58, the duplicate bundle,
and23477 compiler files/19.5GiB. Only archived0.1.59 remains; installed optimized
0.1.21 is byte-identical. Source and previous evaluation evidence are retained.

Native outer -> inner -> outer acceptance uses two simultaneous Laravel13.37
Composer roots in one workbench workspace, with both caller tabs open. Existing
Navigate menu actions show only the selected project's unique route/view names.
The identical logical route name opens the owning routes/openstorm_scope.php
at3:69; the nested label includes tools/child-🐘. Identical view names open the
owning resources/views/openstorm-shared.blade.php at1:1, with distinctive OUTER
or INNER witness text. Both paths return correctly to the outer context. One
resource engine8428 remains under app8416, with one initialization and no restart.
Normal quit exits both. No-argument relaunch8824 restores the same workspace,
all6 clean tabs and outer Blade1:1. After explicitly visiting both recovered
callers, the inner route still opens inner3:69; a view query from recovered inner
Blade opens inner1:1. One resource engine8833 runs; final normal quit exits both.
Both16-file original sets, lockfiles and Composer autoload-file metadata remain
byte-identical. Post-runtime strict/deep signature, executable/engine/source hashes,
installed21 preservation, single latest archive and absent compiler output pass.
Final source reconciliation allows only acceptance-document differences.

No settings, menus, shortcuts, autosave or debugger behavior changes. Physical
Dock-switch autosave, the old46 declaration-readiness claim and broader context/
framework/performance matrices remain open. All69 statuses stay67partial/
2user-deferred. Evidence: artifacts/checks/slice60-{query-uri-red,query-uri-green,
multi-project-client-red,multi-project-client-final}.log; slice60-{focused,
client-syntax,frontend,sdk-gates,workspace-totals,native-red,native-process,
native-progress,app-build,build-receipt,app-readback}.json; both context receipts;
and artifacts/batches/0.1.59/{build-receipt,retention}.json. Final commit/remote
read-back and clean-worktree evidence go in the ignored delivery receipt.

Slice61 — first grouped-route declaration after restart (2026-11-11,
accepted regression/native slice with unchanged development0.1.59): the old46
record describes a first Cmd+B miss after rename, server restart and file switch;
47 only accepted settled navigation. The new actual-client regression sends
its first intelligence request without hover, completion or readiness polling:
first cold lookup,3unsaved grouped-route renames, first lookup after each of
3restarts, exact Unicode declaration ranges and a genuine unknown-route control.
All7expected targets resolve at5:69; the unknown route returns none. Disk sources
remain original. The scenario passes in15.16s with pinned130. Create its disposable
root using scripts/create-openstorm-route-fixture.py --navigation from the pinned
Laravel13.37 baseline; set OPENSTORM_TEST_ENGINE_LARAVEL_ROOT to the receipt's
project and OPENSTORM_TEST_PHPANTOM_BIN to the packaged resource executable.
The ignore annotation states this fixture prerequisite. Both native/client
copies have95non-vendor/non-Blade PHP files syntax-checked with PHP8.4.23 -n.

Native unchanged0.1.59 first user Cmd+B reaches the grouped route5:69. Existing
ShiftF6 prepares the semantic name; the reviewed3occurrence/2file rename to
nav.billing.receipt persists exactly and both changed PHP files lint. After
Restart while the declaration is active, switching to the caller and the first
Cmd+B reaches renamed5:69. Two more first lookups are issued in the same input
batch as Restart: caller-active, then declaration-active with a caller switch.
Both show Finding navigation target and resolve to renamed5:69 without another
navigation attempt. Native background LSP activity is allowed; the separate
actual-client case supplies the strict no-warmup assertion. Old engines10650,
12562 and12639 exit, replaced by12562,12639 and12881 under app10333. Shared
native Edit Undo after all3restarts restores both targets byte-for-byte; first
original-route navigation succeeds. All12witnesses/Composer metadata are original.
Normal quit exits every generation. No-argument relaunch13781 restores the same
project, two clean tabs and original route5:69. The first caller Cmd+B succeeds;
final quit exits app13781 and resource engine13792. Both12-file fixture sets and
Composer lock/autoload metadata remain unchanged; no environment files exist.

Full source gates pass:5333frontend tests, typecheck, lint, knip, format and
production build; Rust1740passed/65ignored/0filtered in1231.79s; Rust formatting,
post-binding typecheck and all-target Clippy322.18s. The new ignored case was
explicitly executed separately. Runtime/build source hashes, executable/engine
checksums and post-runtime strict/deep signature still match the existing0.1.59
build receipt. Differences are the new regression and acceptance documents.
No new runtime bundle is necessary. Only latest0.1.59 remains; cargo clean removes
21176files/16.4GiB after all gates. Installed optimized21 stays byte-identical.

The old125/46miss does not reproduce in this current bounded sequence. This
result does not establish its historical cause or general instant readiness,
fresh startup performance or parity. Physical Dock-switch autosave and wider
route/framework/project/performance matrices remain open. All69statuses remain
67partial/2user-deferred. Next slice: exercise pending declaration lookups against
source/caret/project changes and assess navigation cancellation from a reproduced
case before implementation. Evidence: slice61-first-lookup-client.log;
slice61-{focused,laravel-fixture,native-fixture,native-renamed,native-restarts,
native-progress,frontend,sdk-gates,workspace-totals,app-readback,retention}.json;
post-binding and compiler-clean logs. Final commit/remote/clean-worktree source
reconciliation is recorded in the ignored delivery receipt.

Slice62 — own delayed declaration navigation (2026-11-11, accepted local
signed development0.1.60):10public stale-result regressions fail before the
correction: source/caret/document/project/active-editor changes, closed view,
Escape, superseding lookup, old errors and prepared chooser activation. They
expose a result lifetime that outlives the existing source/selection-bound progress
hint. A separate early-Escape red case exposes the progress timer appearing after
cancellation. The final68related checks pass, including delayed preview-file reads
and positive fresh/second-row activation. Full frontend5346, typecheck, lint, knip,
format/build and post-binding typecheck pass. An initial full run failed one
unchanged Java template-context assertion; all17isolated cases and two subsequent
full runs pass, including the final callback-release source. Its cause remains
unclaimed and the failed receipt/log are retained. The first focused typecheck
exposed an inferred empty-argument spy tuple; its explicit signature is corrected.

Per-view ownership now captures immutable document text, selection, backend doc/
path, project/opening state and active editor. Sync admission, bounded empty-result
retries, errors, web/file and syntax fallback acquisition, preview reads and
prepared rows/pin actions recheck ownership. Escape in the source editor disposes
its progress timer and refuses the later result. The listener is removed and the
progress callback released after completion; the weak ownership token survives for
chooser callbacks. Existing contribution interfaces, shortcuts, settings, layout,
autosave, debugger and server protocol remain unchanged. This is client-side
result cancellation; the server request still completes within its existing bound.

Native baseline0.1.59 reproduces the focus jump: an owned paused server admits
Cmd+B at caller6:20; Left moves it to6:19 and clears progress; explicit resume
still switches to old route5:69. App76862/server77056 quit normally and all12
witnesses remain original. Total14.193s process pause includes pre-request time;
actual successful late navigation is observed, not a timeout result.

New signed arm64 package225.95s, unchanged PHPantom130. Rust1740passed/65ignored/
0filtered in1238.08s, fmt1.41s and all-target Clippy324.99s pass. The known large-
unwind-table development linker warning remains. Strict/deep signature and source/
engine receipts precede pruning obsolete0.1.59, the duplicate bundle and21499
compiler files/18.6GiB. Only archived0.1.60 remains; installed optimized21 is
byte-identical. Engine provenance/source recipe and earlier evaluation sources/
receipts stay preserved.

Corrected native0.1.60 / one resource engine40417 under app40379: fresh Cmd+B
opens route5:69. During the identical owned pause, Cmd+B shows progress and Left
moves6:20 ->6:19. A trace witness records valid nonempty definition request29 reply
for route5:69 at5.604832s, after the AX-confirmed change and before10s timeout;
no definition cancellation RPC was sent. The editor stays at caller6:19 without
late target or error; fresh Cmd+B then opens5:69. A second pause shows progress,
Escape clears it without moving6:20, and valid request49 reply arrives at5.709874s.
The editor remains at caller6:20; another fresh Cmd+B succeeds. Both helpers resume
explicitly, not through watchdog recovery. The trace timestamp bounds the prior
AX event from above while ps still shows the owned server stopped; the valid reply
arrives afterward. Protocol locations use UTF16, including Unicode before target
column68. Only the disposable synthetic project is traced; extracted witnesses
retain URI/range/timing rather than source bodies.

Current RoutesConsumer8:59 opens the standard2target chooser with source previews.
Down/Enter selects /openstorm/b, opens routes/openstorm_refactor.php9:72 and proves
fresh row activation still works after normal picker closure. The same engine and
one initialization span all cases. Normal quit exits both. No-argument relaunch
40958 restores the same project/all4clean tabs/refactor9:72; recovered caller6:20
fresh Cmd+B opens5:69. Final quit exits app40958 and resource engine40968.
Both12-file fixtures/Composer lock/autoload metadata stay byte-identical and all95
non-vendor/non-Blade PHP files per copy lint with PHP8.4.23 -n. No environment files
exist. Post-runtime strict/deep signature, executable/engine/source hashes, single
latest archive and absent compiler output pass. Only final acceptance documents
differ from the built source snapshot.

Physical Dock-switch autosave, broader dependency/configuration/server-generation
freshness, contributed-definition/web fallback native matrices and server-side
cancellation/performance remain open. All69statuses stay67partial/2user-deferred.
Evidence: slice62-navigation-{expanded-red,final}.log; slice62-early-escape-red.log;
slice62-{focused,frontend,frontend-attempt1,frontend-attempt2,sdk-gates,
workspace-totals,app-build,build-receipt,native-red,native-progress,app-readback}.json;
slice62-fixed-{caret,escape}-{suspend,resume-proof,wire}.json; both fixture receipts;
and artifacts/batches/0.1.60/{build-receipt,retention}.json. Final source/commit/remote/
clean-worktree reconciliation is stored in the ignored delivery receipt.

Slice63 — reject stale backend navigation inputs (2026-11-11, accepted local
signed development0.1.61): the shared LSP manager
now snapshots open-buffer inputs attached to each serving generation. Definition,
declaration, implementation and type-definition responses recheck buffer identity,
version, language, immutable text and attachment membership before conversion and
again before returning locations. Stopped generations and newly opened/closed
relevant buffers invalidate the request. Bulk implementation requests also refuse
the entire mixed-revision batch. Cancellation is quiet and prevents a stale reply
from activating syntax fallback. Only metadata and text Arc references are copied;
this does not hash or clone whole buffers or infer a dependency graph.

Protocol red:16changed-input cases across4navigation kinds, a successful reply
from a stopping server and a mixed implementation batch are incorrectly accepted
(3failed/1passed). The correction passes all4new tests and all89mock-server tests.
Stable sources, unrelated documents without this server and servers that do not
send open/close synchronization notifications retain successful navigation.

Native baseline0.1.60 reproduces with the unchanged resource engine130 behind a
transparent owned-fixture transport relay. Its real nonempty definition reply for
route5:69 is computed before a right-pane newline edit; dependency version0->1 is
forwarded before the unchanged response is released at2.694491s. The unchanged
caller is refocused at6:20 before delivery. The old app activates blank target5:1;
fresh Cmd+B correctly resolves6:69. Native Edit > Undo and File > Save All restore
all12original witnesses; CmdQ exits app46197/relay46440/engine46444. Automated
CmdZ did not restore the edit in this run and a quick-definition popup was
visible afterward; its cause and shortcut Undo acceptance are not claimed. No environment files are present. The relay supplies no semantic
result and makes no production engine/default/profile changes.

Full frontend5346/typecheck/lint/knip/format/build and post-binding typecheck
pass. Unfiltered Rust1744passed/65ignored/0filtered in1198.19s, fmt1.38s and
workspace/all-target Clippy with warnings denied318.91s pass. Signed arm64
packaging succeeds in218.13s. The known large-unwind-table development linker
warning and Java8 toolchain warnings remain; neither is claimed fixed. Engine130
and its provenance/source recipe are unchanged.

Corrected native0.1.61 under app8149/relay8183/resource engine8185: initial fresh
Cmd+B opens5:69. The relay holds PHPantom's actual nonempty old definition result;
a right-pane edit moves the declaration to6:69. Dependency version0->1 is sent
while the response is held; the left caller's original text/caret6:20 is restored
and AX-confirmed before an explicit release marker. The unchanged response is
released at5.869739s, after that timestamp and before10s timeout, without watchdog
recovery. The app stays at caller6:20 with no late target/error/hint. Fresh Cmd+B
then opens the correct6:69. This distinguishes the backend correction from the
preceding slice's active-view guard. The relay does not invent semantic results.

Native Edit > Undo and File > Save All restore the source; fresh Cmd+B resolves
5:69. Native Restart replaces relay8183/engine8185 with8199/8200 under the same
app; the old pair exits and fresh navigation still resolves5:69. Normal CmdQ
exits the app and replacement pair. Explicit-profile no-argument relaunch8213
restores the same project,2panes/3clean tab instances and active target5:69.
Recovered caller6:20 Cmd+B resolves5:69; final normal quit exits app8213 and
relay8222/resource engine8223. Physical Dock-switch autosave remains unverified.

Post-runtime strict/deep signature, executable/engine/build-source hashes and
installed optimized21 integrity pass. Both12-original-file fixtures, Composer
lock/autoload metadata and all95non-vendor/non-Blade PHP syntax checks per copy
pass; no environment files exist. Packaging prunes obsolete0.1.60 and the duplicate
SDK bundle, then removes23455compiler files/19.5GiB. Only archived0.1.61 remains;
compiler output is absent. Sources, tools, engine provenance, earlier evaluation
source/evidence and installed profiles remain preserved. Only final acceptance
documents differ from the built source snapshot.

Closed disk files, configuration epochs, the final IPC-to-activation interval
and broader native language/server matrices remain open. All69area statuses stay
67partial/2user-deferred; debugger scope is unchanged. Evidence:
slice63-navigation-{red,green}.log; slice63-no-sync-control.log;
slice63-mock-server-final.log; slice63-{focused,frontend,sdk-gates,workspace-totals,
app-build,build-receipt,red-native-progress,native-progress,app-readback}.json;
slice63-fixed-refocus-proof.json; both fixture receipts and relay event logs;
and artifacts/batches/0.1.61/{build-receipt,retention}.json. Final exact commit,
remote ref, clean-worktree and documentation-only source reconciliation are
stored in the ignored slice63-delivery.json receipt.

Source integration (2026-11-01, verified source checkpoint): integrates all 240 new
commits from `l0ng-ai/openidea:main` at `041e7210` into
`codex/openstorm-php-laravel`, based on pushed OpenStorm `82d1313a`.
The 38 conflicted paths combine upstream cancellation/incomplete rename results,
interactive code actions, project/workspace trust, native window handling,
custom-document closing, and focus disk refresh with OpenStorm's guarded PHP
edits, revision-bound overwrite confirmation, serialized Save All decisions,
reviewed undo/redo, project-model reset, branding, and disabled updater.
PHPantom remains `0.11.1-openstorm.130`; no engine source or binary changes.
Upstream features are inherited code, not new 69-area acceptance claims or
independent debugger expansion. At that source checkpoint, existing bundles
and the installed app were preserved and merged native checks had not run.
Slice54 above supplies the later native smoke and latest-only archive policy.
Integration regressions reproduced and fixed: lazy language loading could cache
an old project model after replacement; delayed old-server shutdown publication
could wake a newer restart as unavailable. The corresponding deterministic red
checks fail before the fixes and pass afterwards. The request-documentation live
control passes all three restart cycles after the shutdown-publication fix.
PHP checked rename now accepts upstream request cancellation and preserves
incomplete-result metadata; its cancellation regression prevents an unchecked
fallback. The host regression checks PHP Optimize Imports through the new interactive
action executor with a guarded plan. Non-cancellation Save All failures still allow the
remaining files to save, while cancelled overwrite decisions stop the operation.
Development session identifiers use the OpenStorm namespace too.
Properties saving preserves upstream ASCII escape spelling without bypassing
revision-bound overwrite confirmation; all35 document cases pass, including a
new stale-confirmation/escape-preservation regression.

Observed source gates: **5,321 frontend tests; 1,738 Rust workspace tests,
60 ignored, zero filtered; actual PHPantom client24/host5; browser64 passed,
one existing Search Everywhere file-opening case skipped**. Type checking,
frontend lint, Knip, frontend build, repository formatting, Rust formatting and
all-target workspace Clippy with warnings denied pass. Bindings were regenerated
by the workspace export test and checked by the frontend afterwards. Frontend
lint/build retain inherited nonfatal warnings; the native debug binary link
reports a large unwind-table warning. Cargo Nextest/Cargo Deny are not installed
and were not run; cross-platform/musl CI and a new packaged native UI pass are
not claimed. Whole-area ledger statuses remain67partial/2deferred.

Evidence: `artifacts/checks/upstream-2026-11-01-receipt.json`,
`upstream-2026-11-01-client-host-final.json`, `*-workspace-final3.log`,
`*-clippy-final3.log`, `*-frontend-final.log`, `*-e2e.log`, and deterministic
`*-restart-red/green.log` / `*-lazy-project-red/green.log`. Git history and the
source receipt record the final merge/push ref. No environment file was changed
in the source diff; only previously approved suite-owned fake test files were
used during checks. Obsolete standalone compiler objects were pruned with
frozen metadata, inode/hardlink/header/hash checks while owned build schedulers
were paused and compiler jobs quiescent; all schedulers resumed. Applications,
archives, engines, source, fixtures, and evaluation deliveries were preserved.

Packaged checkpoint (2026-10-31): signed development **0.1.53**, bundled engine
**0.11.1-openstorm.130**, finishes the current tested development checkpoint.
Full engine18,408+FIFO child/17ignored, actual client24/host5, frontend3057,
workspace1321/49ignored/0filtered, typecheck/lint/build/fmt/Clippy pass. Native51
component/slot/bag/frontend-binding cases and native53 discard/reopen, exact
Unicode definition,8-tab noargs recovery, fixture/default restoration, signatures
and normal app/server exit are observed. A physical user-controlled Dock app
switch remains unverified: the user is not at the Mac; automatic persistence was
observed independently during waiting. No complete69-area or release acceptance
is claimed. All69 statuses remain67partial/2deferred. Installed21 and prior bundles
are preserved. Local candidate: `artifacts/batches/0.1.53/OpenStorm.app`.
The packaged checkpoint source was committed/pushed as `82d1313a`;
`project-discard-0.1.53-delivery.json` records its final ref verification.

Earlier checkpoint (2026-10-31): signed development **0.1.49**, bundled
PHPantom **0.11.1-openstorm.127**, accepts the narrow inactive-tab diagnostic
refresh and recursive view-data check. The editor now refreshes saved diagnostic
state even when the current diagnostic list is empty, clearing old underlines,
overview marks and counts after a tab has been inactive. A real EditorView
regression failed before the fix and passes afterwards. Native testing reproduces
missing view data, clears it while the leaf is inactive, then reopens the leaf
with no stale underline/error count; completion, definition, settled restart and
normal quit/relaunch pass. All fixture originals and temporary setting defaults
were restored. The installed optimized 0.1.21 and earlier builds are preserved.

Local app: `artifacts/batches/0.1.49/OpenStorm.app`. Native evidence:
`artifacts/checks/view-data-cycles-0.1.49-native-result.json`.
Checks: **3,050 frontend tests; 1,321 workspace tests with no filters; all 19 live
PHPantom client and five host scenarios**. The workspace retains 44 ignored tests;
the separately executed client/host cases establish only their own acceptance.
Both newly approved disposable environment-file process tests passed. Type
checking, lint, builds, formatting and Clippy passed. Engine 127 is unchanged and
retains its prior 18,387-test source gate plus the separate FIFO child.

The 0.1.48 custom-root/package/closed-config view batch and 0.1.47 route/history/
save-conflict batch remain accepted within their recorded scopes. Remaining
limits include generic HTML fallback when a Blade PHP type is missing, broader
component/shadowing matrices and an initial frame-only window-control attempt in
0.1.49. A fresh comparison process and a subsequent normal no-argument relaunch
rendered correctly; that initial anomaly remains unexplained and is not erased.
All 69 areas remain visible: **67 partial and two user-deferred**. No blanket
parity, installation, public release, notarization or new push is claimed.

## Contract

Preserve OpenIDEA layout, default keymap, autosave and debugger. Use separate
cz.thinky.openstorm application data; retain .openidea project settings.
Preserve the released PHPantom 0.11.1-workbench.114 foundation and its provenance.
Use the checksum-verified bundled PHPantom with pinned source and patch provenance;
preserve builtin documents and PHP target context. Acceptance belongs to the
observed package and workflow, not to capability flags or a source test alone.
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

Next FEAT-030 corpus started after the 0.1.43 history gate. A new ignored/live
OpenStorm-client regression covers grouped optional parameters, named argument
keys, resource parameter overrides, controller groups and unsaved URI changes.
The pinned bundled .123 engine passes index parameters and completion for
invoice, optional tab and custom resource record. The controller comparison
exposes a real failure: the imported `Routes` facade alias yields no definition;
changing only the facade spelling/import to canonical `Route` resolves the same
controller method in app/InvoiceController.php at line 3. Unsaved-parameter
assertions after that failure are not reached and are not accepted.

The failure is preserved in `route-navigation-next-client-control.log` and
`route-navigation-next-reproduction.json`. The initial run had an incorrect
resource path and is invalid evidence; subsequent corrected-path/control runs
use the actual signed bundle resource. Source inspection finds spelling-only
checks in both controller completion context and controller action span
extraction. Next engine patch must resolve facade identity (aliases/FQN/local
non-Laravel controls) while preserving nested controller groups, then repeat
source/client and native gates. This next route corpus remains unaccepted;
0.1.43 history acceptance and the installed optimized app are unchanged.

Route-controller engine correction in progress. The expanded `--navigation`
disposable fixture adds controller-group actions, optional/named/resource
parameter keys and cross-file duplicates; every new PHP file was syntax-checked
and the existing pinned Laravel baseline/autoload checks passed. Native signed
0.1.43 reproduces the .123 defect: CmdB at openstorm_navigation.php5:53 shows
Cannot find declaration to go to. No fixture bytes changed and normal quit
exited the app. Evidence: route-navigation-baseline-043-native-result.json.

A separate working engine tree preserves the verified .123 source. Focused
source regressions cover alias/grouped-import/FQN action navigation/completion
and unrelated namespaced facade homonyms. Initial source test build/link is
running; this is not engine correction or .124 acceptance yet. The correction
will reuse existing resolved-name/facade identity and controller-chain helpers
rather than add another resolver. Native .43 remains accepted for history;
next route-navigation batch remains unaccepted until its gates pass.

The two new engine regressions fail on unchanged .123 as expected: alias
navigation is absent, and an imported unrelated Acme Route incorrectly gains
a controller target. `route-controller-124-red.log` records both failures.
The fix now resolves action receivers at their existing parsed-name offsets,
validates controller group roots through the shared facade helper, and threads
those resolved names through nested action scanning. Completion reuses the
same controller-chain extraction instead of its former duplicate resolver.
The working tree passes all-target release Clippy with warnings denied and
formatting after Clippy. The inherited rust-objcopy/libLLVM stripping warning
remains a tooling warning, not a new source lint failure.

Patch 0124 is recorded with checksum in engine/source.json; source recipe .124
replays cleanly into `1756b1f920c1-d25f6d9b17b0-verified` and its changed files
match the working fix. Series SHA-256:
d25f6d9b17b014e377ee10de68ae06c9c8253855d112ccd68846ed059d8923b7.
The focused corrected-source test build is running in the existing engine
compiler cache (`route-controller-124-green.log`); no green/native acceptance
is inferred yet. The bundled resource/provenance remains .123, and all old
source trees, binaries and apps are preserved. Full approved synthetic engine
suite, real client/host gates, new package/native checks and root batch gates
are still required before promoting this correction.

Corrected-source focused controller corpus is green: nine tests pass, including
all former canonical cases plus imported/grouped-import/FQN action completion
and definitions, and unrelated namespace/import homonym refusals. The archived
candidate .124 executable reports its exact version and has SHA-256
686a2a6aa2c0e856ea9ff433a4a8a6d242ae893dce6bb26554cc2fd008319b62;
its recipe, licenses and toolchain are preserved under artifacts/engines.
The previously failing OpenStorm client case passes, including its previously
unreached unsaved URI-parameter checks and unchanged disk assertions.

All 15 live client cases then pass with the explicit Laravel12 model, owned
Laravel13/view fixtures and candidate executable (89.75s). The first broad run
omitted fixture paths and had seven setup failures; it is preserved as invalid
broad-acceptance evidence, not a feature regression. The configured final result
is `route-controller-124-openstorm-client-fixtures.log`. All owned navigation
fixture files still match their original bytes. The full engine suite is now
running with the already approved suite-owned synthetic env scope; live host
checks are also running against the .124 archive. Those gates, packaged native
before/after verification and final root checks remain open. No promotion or
installed-app replacement has occurred; .43 remains the accepted history build.

All five live host cases pass against archived .124 (36.48s), including guarded
checked rename with closed declarations, grouped PHP/Blade route edits,
resource-first-open navigation and framework-source restart recovery. Every
owned navigation fixture source is restored after those checks. The engine
full-suite process remains live in its existing compiler cache; resume that
process/log rather than restarting it. Packaged .124 native acceptance and the
final root batch gates remain pending. This is a source/candidate checkpoint,
not a promoted release or full FEAT-030 acceptance.

0.1.44 preparation: all four version sources / 34 workspace lock entries now
use the unique development version, with no other lock changes. Frontend
3,044 tests / 352 files, type checking, lint and build pass; inherited lint /
bundle warnings remain. The final workspace tests/fmt/Clippy/bindings pipeline
is running with the same two separately pending SDK env cases excluded.

The first full .124 engine source run failed two inherited Composer helper
assertions after returning valid definitions. Exact same executable controls
prove a temporary-path setup boundary: with `/var` TMPDIR both strict URI
comparisons fail; with canonical `/private/var` both pass. The source helpers
construct expected URIs from the temporary path while backend locations are
canonical. No production or test source was changed to suppress the failures.
The obsolete default-path isolated-rerun process group was verified and
cancelled for that confirmed setup error; terminal exit143 is recorded. The
full suite was restarted with explicit canonical TMPDIR, .124 version and
locked release profile; poll the existing canonical-suite process, not the
failed/or cancelled runs. Reproduction: route-controller-124-path-controls.json.
The canonical invocation is now documented in engine/README.md. The original
full gate remains a recorded failure; no passing full-engine result is claimed
yet for the corrected run.

Build headroom: 524 obsolete regenerable SDK .rlib/.rmeta files (4,209,546,607
logical bytes) and the obsolete 2,123,932,928-byte native .43 link archive were
removed after validating their target paths, retained newest-two policy and
idle SDK compilers. Apps, source, fixtures, engine caches/archives and earlier
Workbench/evaluation delivery artifacts were excluded. Reclamation was delayed;
initial free-space readings did not prove recovery. Subsequent filesystem
reading showed 12GiB available. The purgeable local Time Machine snapshot was
only inspected and was not deleted. Detailed selection/hash/space receipts:
route-navigation-0.1.44-sdk-cache-pruned.json and native-link-cache.json.

Final .124 source gates pass with the explicit canonical temporary root:
18,381 tests passed / 17 ignored, plus the separate FIFO child check. Both
initially failed path assertions pass in this full rerun. Verified replayed
source passes all-target release Clippy with warnings denied, fmt and source
fingerprint re-verification. PHP example lint (12 files), executable assertions,
Laravel Demo syntax and static analysis all pass; the analyzer returns the
expected three intentional Demo.php errors at 731/731/951. The full-suite
compiler-cache executable matches the archived candidate SHA-256 exactly.
The initial noncanonical-temp gate remains recorded as failed, with direct
same-binary controls and the corrected full rerun retained.

Final .44 source gates: frontend 3,044 / 352 files, type checking, lint/build,
workspace 1,318 passed / 40 ignored / two separately pending SDK env cases
filtered, fmt, all-target Clippy with warnings denied and binding export pass.
The exact .124 resource and provenance are staged and checksum-verified;
previous .123 provenance, binary and all apps remain preserved. A second
regenerable SDK-test static link archive was removed at rest before Tauri's
resource-triggered replacement to avoid its 2.1GB peak; receipts retain its
hash and free-space readings. No app/delivery artifact was removed.

Signed arm64 development .44 is preserved at
`artifacts/batches/0.1.44/OpenStorm.app` (330.27MiB), executable SHA-256
adfa476a130b6407d798bf48d485a1779faddd0f8587c1abb6228a1dfe063523.
Actual native CmdB on the imported Routes controller-group action now reaches
NavigationController.php4:21; .43 reported Cannot find declaration at the same
fixture position. Settings query CmdZ/CmdShiftZ also passes.

**.44 remains diagnostic / route batch unaccepted.** Native CtrlSpace at5:53,
inside existing action 's|how', auto-applies its sole method candidate and yields
'showhow'. Undo restores the original action and disk stays unchanged; CmdS
clears the temporary modified state. Inspection confirms the quoted-controller
provider returns plain insertText without a replacement edit, while OpenStorm's
inherited default intentionally inserts over the prefix (Enter) and reserves
identifier-suffix replacement for Tab. Fix the provider's quoted-action range
without changing these editor conventions, and add a commit-result regression:
label-only completion tests did not establish correct insertion. Parameter,
duplicate, rename, restart and relaunch native checks remain pending in the
corrected next candidate. Evidence: route-navigation-0.1.44-native-result.json.

The disposable profile's switch-app autosave true / idle false defaults were
restored. Normal quit exits app and server; all 12 owned source files remain
original and strict signature passes. Installed optimized .21 and its signature
remain unchanged. No publishing, new push or promotion of .44. All 69 ledger
areas remain visible at 67 partial / two user-deferred.

Quoted-action .125 correction in progress. A public-handler regression applies
the returned completion edit to real PHP source with a Unicode URI. Against
unchanged .124 it executes and fails with showInvoicehowInvoice, matching the
native .44 failure. Initial unqualified --exact invocation selected zero tests
and is invalid as red evidence; the fully qualified case was run directly from
the just-built executable and executed one failing test. A redundant owned
Cargo rebuild was cancelled after the intended direct regression completed;
its terminal143 is not a test result. Receipts: route-controller-125-red-direct.log
and red-result.json.

The actual OpenStorm client regression also fails against .124: it receives
None where a whole-method UTF16 replacement range is required. It now verifies
raw and resolved edits, plain-text insertion and a Unicode URI before the field.
The provider fix retains the existing insertText and adds a standard TextEdit
covering the complete identifier inside the quotes, including a Unicode suffix.
It uses existing position conversion and identifier scanning; global Enter,
Tab and single-candidate completion conventions are unchanged. Six commit cases
(empty/mid/end prefix and changed ASCII/Unicode suffixes), before/after resolve,
are included in the handler test. Working all-target release Clippy passes;
formatting ran afterward.

Patch0125 and recipe .125 replay into
`1756b1f920c1-c56c54f64023-verified`; series SHA-256
c56c54f6402325af9178ed27177df5ff9efe1e4cdee7e1c82ab34c05bf9921f1.
All ten controller source tests pass. The full pinned source suite passes
18,382 tests / 17 ignored, with its separate FIFO child passing one test.
Pinned-source Clippy, formatting and source fingerprint checks pass. Actual
OpenStorm client tests pass15/15 and host tests5/5; PHP example syntax and
assertions pass, and the analyzer retains its three expected intentional errors.
Canonical TMPDIR is used for macOS URI equality, as documented above.
The .125 resource checksum is21a7b6ed5e6040898b8d8922040b2057f03b5f4a6ea7c3487b524d4036d61ba8.
.45 application gates pass: frontend3044tests, type checking/lint/build,
workspace1318passed/40ignored/2filtered, formatting and Clippy. The workspace
executes binding export; no schema changed. The same two SDK environment cases
remain excluded pending their separate approval; engine fixture approval does
not extend to them. Native .45 acceptance remains pending. A new .45 disposable navigation fixture is syntax and
pinned-baseline/autoload checked. The .44 archive/receipts remain preserved.
A post-native regenerable static link archive was removed at rest because
free space was1.7GiB; only that compiler cache was removed, restoring build
headroom without changing any runnable app, source or provenance. Receipt:
route-navigation-0.1.44-post-native-link-cache.json.

Native .45 / engine .125 diagnostic (2026-10-31): signed development app
`artifacts/batches/0.1.45/OpenStorm.app` launches from its isolated owned profile.
CtrlSpace at5:56 after the Unicode route URI replaces the entire method atom,
but the client then appends call parentheses: `'show()'`. This is persisted and
recorded in route-navigation-0.1.45-method-completion-failure.json; it is not
accepted. CmdZ plus CmdS restores all12owned fixture files. CmdB still navigates
to NavigationController.php4:21. Normal quit exits app/server. No release or push.
The frontend callParens helper treats method-kind plain text as a call without
checking whether the replaced atom is surrounded by quotes. A focused client
regression and bounded quoted-atom guard are the next gate. Remaining native
parameter, duplicate, checked rename, restart and recovery checks are pending.
Workspace binding export passed in the full workspace gate; no separate
feature-variant export rebuild was required. SDK compiler-cache pruning receipts
record .45 library/test retention and exact static archive removal at idle;
all installed, archived, Workbench and evaluation deliveries are preserved.

Client quoted-action correction for .46: the production completion-application
regression reproduces .45's exact persisted `show()` failure before the fix.
callParens now skips atoms whose replacement begins immediately after a quote.
The scope includes single/double/backtick quoted symbols and unfinished quotes;
existing ordinary-call, parameterless-call, snippet, Enter/Tab, import and
late-edit tests remain in the same suite. The focused32test suite passes after
the fix. PHPantom .125 remains unchanged and checksum-verified, so its completed
source/client/host gates are reused. Fresh .46 application/native gates are
running; no native success is inferred. All69areas remain visible.

.46 source gates complete: frontend3046/352files, final focused completion32,
type checking, final lint, build, workspace1318/40ignored/2separately pending SDK
env cases filtered, binding export, formatting and all-target Clippy pass.
Initial lint caught only three unnecessary test-literal escapes; these were
removed and the affected suite/lint/build rerun successfully. Existing unrelated
lint/Rollup/canvas warnings remain recorded. Native packaging is next.

.46 packaging receipts retain two storage failures (initial bundle copy and
first retry static-archive replacement) followed by a successful retry2.
No source changed between attempts. Only regenerable exact workspace compiler
libraries/test executables and the idle static link archive were pruned with
selection/stat receipts; installed and archived apps, engine archives, source,
fixtures, PHP Workbench and earlier evaluation deliveries were preserved.
Tauri retry2 completed successfully; native acceptance is still pending.

Native .46 diagnostic completion (2026-10-31). Signed app executable SHA256
8a8482aac588b73bfb68cca328b04c18a6f37ea2c14e6860e0d2899c2441d8c0;
PHPantom .125 checksum unchanged. Midword sole-candidate CtrlSpace keeps show
plain inside quotes after the Unicode route path. Empty-prefix choice index()
commits exactly index, preserving all surrounding source and11otherfiles; Undo
restores. CmdB reaches controller4:21. Settings focused query Undo/Redo passes.
Native required invoice, optional named tab and custom resource record parameter
keys persist exactly. Unsaved URI invoice->receipt supplies receipt completion
in another unsaved file while both disk files stay at the saved keys state.
The static Laravel Routes action discovers the grouped Unicode target5:69 and
both duplicate targets8:80/4:80; selecting the second opens the correct file.

Grouped route rename previews3occurrences/2files. Cancel writes nothing; Apply
changes the leaf and two consumer names, preserving controller action show,
controller declaration, resource route, duplicate names, Unicode URI and keys.
Native menu Undo restores both files; keyboard Redo from the consumer reapplies
both. Renamed route declaration navigation reaches5:69. Restart exits the old
server and initializes bundled125; method completion/controller navigation pass.
The first renamed-route lookup after switching files reports no declaration;
a settled retry succeeds. Immediate readiness is not claimed.

Failure: changing only the second duplicate registration to nav.unique in an
unsaved buffer clears its own duplicate_laravel_route warning, but the unchanged
open sibling continues reporting the old duplicate after repeated switching.
Disk witnesses remain unchanged. This blocks acceptance; .46 is not promoted.
Record: route-navigation-0.1.46-duplicate-diagnostic-failure.json. Next: add an
actual publication regression and fix cross-file route-diagnostic refresh.

The synthetic profile restores switch-app autosave true and default idle false;
its settings.toml contains only files.autoSave=true. Normal quit exits app and
server. After restoring only the12owned fixture files, no-argument relaunch
reopens the project/four tabs and reads original disk source; final normal quit
exits app/server, all12files stay original, strict bundle signature passes, and
installed optimized21 checksum/signature remain unchanged. No new push/release.
Native result: artifacts/checks/route-navigation-0.1.46-native-result.json.
All69areas stay visible at67partial/two deferred. Wider platforms, real services,
performance parity and the two separately pending SDK env tests are unaccepted.

.126 diagnostic-publication regression started in ignored working clone
`artifacts/phpantom/route-diagnostics-126-work` of immutable125verifiedsource.
The test extends the existing real ProjectRouter transport harness and waits
for new complete, versioned, sequenced publishDiagnostics on the unchanged
sibling after an unsaved edit. Its initial warning must exist first; collection
helpers are not called directly. Current engine logic and bundled125stay
unchanged; the red test build is pending in route-diagnostics-126-red.log.
No test result or source126 acceptance is inferred from compilation.

Actual OpenStorm client .126 regression now executes and fails on the signed
.46 bundled125engine: both initial duplicate warnings exist, and the edited
registration clears its own warning, but the unchanged open sibling receives
no fresh diagnostics. The manager shuts down before the failing assertion.
Receipt route-diagnostics-126-client-red-result.json. The engine transport case
also executes one failed timeout; detailed stage instrumentation is still
compiling, so its precise timeout stage is not yet claimed as red evidence.

The detailed engine transport red case now confirms both initial warnings and
edited version2complete publication before timing out on the unchanged sibling.
One actual sourcecase executes/fails101; the old uninstrumented timeout remains
preserved. The working126fix hooks guarded AST publication and reuses the
existing native diagnostic queue: only open route sources and semantic route
callers are queued, with standard/provider/known include provenance, excluding
changed targets and unrelated or closed files. The route catalog is invalidated
at accepted publication. A scope-queue regression covers standard/provider
sources and unrelated/closed callers; the transport test adds close/reopen
recovery. Focused green build is pending. No source126pin/bundle/native acceptance.

Working126focus now passes: the real transport case completes6.99s across
add/move/remove and unsaved close/reopen, retaining disk originals. The actual
OpenStorm client case passes5.99s against the working126executable. These are
working-source checks; the queue-scope unit, Clippy/fmt, pinned-source rebuild,
full suite and .47native gates remain open. Root bundled125is unchanged.

Source126is pinned by patch0126 (SHA256
7af10cad10621dd6e7bb64d7d37c43dec89e3cc532b775873cff07f580b24e37), series
beeee3552967208dffe35bb32d7c1af1aa0e6badc73aac337fc888eea22235bc,
reconstructed independently at1756b1f920c1-beeee3552967-verified. The working
scope-queue unit passes; release all-target Clippy--fix passes and formatting
runs afterward. Full pinned-source suite/Clippy/fmt/fingerprint is running.
Final transport assertions add moved related-location and new opening-identity
checks, pending the full pinned run. Resource125stays unchanged until source
and SDK pipelines finish at a safe boundary.

The .46 shared Redo also displayed a negative selection length. A focused
frontend regression reproduces a compact reviewed replacement mapping an
unchanged backward selection to from84/to34. Shared reviewed transactions now
refine only otherwise-inverted mappings with bounded size/time using the existing
merge diff utility. Large/over-budget replaced selections collapse safely;
valid ordinary mappings stay unchanged. Six focused reviewed-history tests pass,
including selected invoice text/backward direction across apply/undo/redo and
a large-change fallback. .47native verification is still required.

Compiler-space recovery:640older workspace-only Mach-O object files in40groups
were removed while retaining the newest two groups. Only temporary SDK inputs
were removed; engine compilation used a separately verified target. Logical
selected bytes8,748,652,720; immediate free space rose about1.8GB, and laterdf
showed12GiB after delayed reclamation. No app/source/fixture/engine/Workbench or
evaluation delivery changed. Selection/pruning receipts use
route-navigation-0.1.47-sdk-object-cache-selection/pruned.json.

App47versions agree across34workspace packages. The initial version-only count
also matched the already-pinned num-integer0.1.47, so the receipt now verifies
workspace package identity and proves all nonworkspace lock records unchanged.
Application gates are running; the same two separately pending SDK environment
cases stay filtered. No .47packaged/native acceptance is inferred.

Pinned126source gate completes:18,384passed/17ignored, plus separate FIFO child1;
all-target release Clippy, fmt and repeated source fingerprints pass. Engine
SHA25634d2de51be91e8e3c48f850b4314542bce8ff955376a3de0b27c63c1a11d5b69;
archive/provenance/licenses are retained, and the application resource now
matches this exact binary. All16actual client cases and5host cases pass; the
12owned47fixture sources return to original state. The12PHPexamples lint,
explicitly enabled zend.assertions=1 execution and Demo syntax pass. Analyzer
reports exactly the three intentional Demo errors at731/731/951 (expectedexit1).
The inherited LLVM stripping warning is retained; it is not a Rust code warning.

The selection guard's CRLF fixture initially only varied a label, so that run
is invalid as CRLF evidence. The corrected real lineSeparator facet exposes
wrong document text before correction: state.sliceDoc serializes CRLF while
ChangeSet offsets count internal LF. Using state.doc.sliceString corrects the
coordinate basis. Final focused history7and full frontend3049tests pass, with
final typecheck/lint/build. New engine126native acceptance is still pending.

Tools metadata regression executes against the real checked executable with
an empty PATH: the old status row reports123while the binary reports126.
Version now derives once from build provenance, avoiding a manually drifting
label. All three bundled-resolution/override/version cases pass. Only Tools
presentation changed after the16client/5host run; server configuration and the
engine binary remain identical. Final workspace/fmt/Clippy gates are rerunning
for this Rust change; previous intermediate workspace count is1318/41ignored/
2separately pending env tests filtered. No47bundle/native acceptance yet.

Final SDK gate storage recovery: the first final rebuild stopped at ENOSPC
before running tests. At an idle boundary, reviewed 4,996 pre-Oct31 root
workspace Mach-O compiler objects (7,257,093,512 logical bytes) and the exact
regenerable static archive were removed. Recent objects, libraries, executables,
apps, engine sources/archives and fixtures remain preserved. Physical free space
was measured at13GiB before retry, rather than inferred from deletion. Receipts:
route-navigation-0.1.47-old-object-cache-selection/pruned.json. The retry uses
new logs; the failed gate is retained and is not acceptance evidence.

Final47SDK retry passes:1319tests/41ignored/2separately pending environment
cases filtered, binding export, formatting and workspace all-target Clippy
-Dwarnings. Failed ENOSPC log remains retained. Final frontend3049/typecheck/
lint/build is unchanged. Development Tauri packaging is now running; native
acceptance still pending.

Signed development47 native observations: quoted midword controller completion
keeps plain show; empty-prefix index choice persists exactly and keyboard Undo
restores it. Controller alias CmdB reaches4:21. Settings field CmdZ/CmdShiftZ
and native PHPantom126version/resource path pass. Invoice/tab/custom record
key completions persist exactly. Unsaved URI invoice->receipt propagates to
completion in another unsaved buffer without disk writes; overlays restored.
Find Action discovers Laravel Routes; grouped Unicode target5:69 and both
duplicate targets8:80/4:80 open correctly. Both initial duplicate warnings
clear after only the second declaration changes unsaved, and Undo republishes
both warnings. Exact disk witnesses remain unchanged.

Checked rename Cancel writes nothing;3occurrences/2files apply exact changes.
Native Edit Undo from declaration restores both files; Redo from caller reapplies
both. The unchanged backward invoice selection remains7characters across all
three transitions, with10physicalCRLF breaks intact in the caller. Newly renamed
route CmdB reaches5:69. Explicit restart exits15169and initializes126as15551;
quoted method completion and controller/settled route navigation pass. No instant
startup readiness claim; earlier46transient lookup limitation remains tracked.

Save-conflict native check: unsaved local comment plus separately changed owned
route disk file prompts File changed on disk before overwrite. Cancel preserves
exact external bytes and local overlay. Reload refuses dirty state; explicit
guarded-close Don t Save then reopen shows external witness. Defaults restored
to focus autoSave=true/idle defaultfalse. First normalquit exits app15139and both
engine generations. Only12owned fixture sources restored after exit; no-argument
relaunch reopens same project/fourtabs with original route/caller sources. Final
secondquit/signature/installed-app checks are pending before batch acceptance.

Final47native batch is accepted within its recorded slice. Second normalquit
exits the owned relaunched app16584/server16592. A subsequent UI read reopened
an extra candidate16655/server16934; resetting the stale CUA binding and normal
CmdQ without a post-quit UI read exits both. All observed app/server generations
are absent. This tool lifecycle correction is recorded separately from app quit.
All12fixture sources original; focus autosave persistedtrue/idle defaultfalse;
strict candidate/installed21signatures pass and installed21hash is unchanged.
First profile launch IPC3,139ms, warm noargrelaunch847ms on122filefixture.
Three desktop-idle ps samples over10.026s show parent RSS193,744–193,904KiB and
server134,064KiB; parent CPUtime increases0.07s and server stays3.10s. These are
development process snapshots, excluding WebContent/OS cache and wider workloads.
No memory/performance parity or release benchmark claim. Native result receipt:
artifacts/checks/route-navigation-0.1.47-native-result.json. Next work remains
milestone2 acceptance; full69area goal stays active. No new push or installation.

## View roots and package navigation follow-up (in progress)

After accepted47, FEAT-032 broadening starts against its exact bundled126binary.
A seventeenth actual-client scenario passes configured-root precedence, published
package overrides/only-published views, plain PHP templates and unsaved provider
namespace changes without disk writes (initial log). Expanded scenario confirms
unsaved config root switching, then fails cached chooser names after creating a
plain PHP template under a custom themes/ root: direct target resolves but list
omits it after80queries/deadline. Both initial and deadline red logs retained.
Working127source clone preserves immutable verified126. Next: focused engine
watcher regression/fix, then source/client/host/native gates. No127pin/bundle or
view-batch acceptance yet.

View follow-up native comparison: separateLaravel13.37fixture passes88PHP
syntax checks and AppPSR4 mapping, with all environment files excluded. The exact
47/126bundle lists custom-root live-report after external creation and opens
themes/current/live-report.php1:1 in this full-framework run. This is a positive
observation, not the minimal-client failure reproduction. The minimal client
(strategy none) still fails after its80querydeadline; both observations retained.
Diagnostic app normalquit exits;11fixture originals match and only the newly
created owned PHP template is removed after exit. Focused engine red build is
still running; no fix/native candidate acceptance claimed.

Working127engine TDD: the watcher unit fails the first new custom-root name
assertion on126logic (one actual failure/6926filtered), then passes with the
initial fix across editor-owned/closed creation/removal, Blade-over-PHP
precedence/fallback, and cache retention for content edits/unrelated files.
The refined batch implementation reads configured/provider roots once, includes
previously missing configured directories, and reuses canonical-path comparison.
Clippy--fix, fmt-after-Clippy and the expanded working unit are running. Root
client case additionally covers new directories, bounded results, cancellation,
restart and closing provider/config overlays; those later steps remain unrun
until a corrected executable is available. Archive/resource126still unchanged.

Initial127pin: patch0127b28e63c895fc8cb88fa202e66c95f17dc2b28afbc6db97f27dd966e458f753f1,
seriesf9f2c3e61d6d78237595f473ad16f08806dd6a69ada1b95b1eb594c6a9397351,
verifiedsource1756b1f920c1-f9f2c3e61d6d-verified. Its full source pipeline is
running under handle12634; do not amend root source.json until terminal because
the final build-recipe stage reads that pin. This initial pin is not accepted.

An eighteenth actual-client case proves a second view-freshness failure against
the unchanged47/126binary: config/view.php is never opened, initial current-only
view list is confirmed, then external closed config change to alternate root
still returnscurrent-only after80queries/deadline. Original disk restored after
server shutdown. Log:view-config-0.1.47-client-red.log. Working127clone now adds
closed config CHANGED eligibility without a parsed-file prerequisite, retires
the catalog on configuration deletion/creation, retains open-overlay authority,
and handles canonical config aliases. A new backend regression covers these
cases. These amendments are not yet pinned/tested; draft backed up under
view-navigation-127-amended-working-draft.patch. Next: after initial pipeline
finishes, working Clippy--fix/fmt, amend127patch/checksum, reconstruct a new
immutable source identity and rerun full pinned gates plus all18client/5host
cases before resource replacement/native48.

Storage receipts record1,252compiler object groups/2,681hardlink paths and the
exact regenerable SDK static archive removed at idle; all library archives,
apps/executables/source/fixtures/engine/evaluation deliveries retained. Physical
space did not materially increase, so no SDK full-gate/package readiness claim.
Current engine-only pipeline started with5.67GiB measuredfree; SDK full rebuild
should wait for adequate measured headroom. No TimeMachine/snapshot purge.

Initial127source pipeline finishes successfully:18,386tests/17ignored plus
FIFO child1, Clippy/fmt/fingerprint and normal release build. Preliminary binary
SHA2565c6abde3c9c831229b97b9f9d1a3db981c26d06eaa6f8ccfbb1459730cacda4f
is preserved atartifacts/engine-diagnostics/0.11.1-openstorm.127-f9f2c3e61d6d,
with source pin/patch and licenses. Expanded main client case passes7.22s against
this copy across file/root creation/deletion, namespace/config overlays, bounded
results, cancellation, restart and closing overlays. Separate cold config case
fails this initial127as well, so these green source gates do not accept the batch.

Focused native47/126cold-config reproduction now fails too: fullLaravel13.37
fixture syntax86andAppautoload validated; only synthetic test profile code-vision
counts disabled and indexing none. Only bare PHP caller opened; initial Views
listcurrent-only observed, then closed config file externally changed to alternate.
Reopening Views still showscurrent-only and alternate-only filter shows No static
matches found. Log confirms config was never opened. Normalquit exits app21759/
server21792; all6owned source/config originals restored after exit. Synthetic
profile overrides intentionally retained for the corrected comparison; restore
counts defaults after final native acceptance. Receipt:view-config-native-47-result.json.

Amended127working Clippy--fix/fmt pass. Final patch0127SHA256
29437fd3b06e8254f24cc5dbe228f1dc445d83ffd1a16baafa0a7b9a407fd300;
series747e151275eaa1404d7ad3b2025d56ef9ffc9af66f51d80292c8bad619855e82,
source1756b1f920c1-747e151275ea-verified. Final full source pipeline28191is
running; do not change the pin before its final build recipe finishes. Added
cold config regression protects creation/change/removal, open overlay authority
and canonical aliases. Root manifests/34workspace lock packages now48with all
thirdparty lock blocks unchanged. Frontend/typecheck/lint/build pipeline65149is
running. Actual18client/5host, SDK final and packaged48native gates remain open.

Amended127full source gate finishes:18,387passed/17ignored plus FIFO child1,
Clippy/fmt/fingerprint and normalrelease build. EngineSHA256
0ef92bcb1199ea7f054d2d0aa0fcd20ebcf08d575f311514d1438cb2ff8d531c.
The expanded cold-config actual-client case also checks invalid-view diagnostics
appear after the root change and clear after restoring the original configuration;
it passes7.06s. The complete18client cases pass108.59s; all5host cases pass35.84s,
with all12shared47fixture sources original after each stage. PHP examples12lint,
explicit zend.assertions=1 runtime and Demo syntax pass. Analyzer retains exactly
three intentional diagnostics at731/731/951 (expectedexit1). The127archive includes
licenses, provenance, toolchain, build and validation receipts. Resource127now
matches the checked archive; node checksum verification passes. Native acceptance
is stillfalse. All3049frontend/typecheck/lint/build gates pass; inherited warnings
remain. Final48SDKworkspace/fmt/Clippy is running with only the two separately
pending environment cases skipped. No new push/installation/public release.

Final48workspace test run finishes with1,319passed/43ignored/2filtered, nofailed
test groups. The18pinned-client and5host ignored cases were executed separately
and passed above; remaining inherited ignored cases are not accepted by this run.
Only the two pending SDK environment cases were filtered. Finalfmt completed;
Clippy still running. Native48bundle acceptance remains open.

Final48Clippy workspace/all-targets-Dwarnings passes1m57s; SDK pipeline2599
terminal0. sdk-gates.json records exact logs/counts. Tauri debugbundle pipeline17359
started with unchanged runtime inputs. All69area statuses remain67partial/2deferred.

### View roots and package navigation: 0.1.48 accepted locally

The previous 0.1.47/126 cold-config failure is preserved. The corrected comparison
used the same full Laravel 13.37 fixture with `config/view.php` never opened,
indexing set to none, and only the disposable profile's code-vision counts
temporarily disabled. Initial `current-only` changed to `alternate-only` after an
external closed-file change and opened the alternate template at 1:1. Deleting
that owned config selected the default resource-root template; restoring it
returned `current-only`. Runtime logs confirm the config document stayed closed.
Explicit restart replaced server 72343 with 72372 and retained correct lookup.
Both count defaults were restored through Settings. App 72323 and both server
generations exited on normal quit. No-argument relaunch restored the same project,
original caller and three tabs; final quit left no owned processes.

The second native fixture exercised custom-root precedence, published package
card override, a published-only banner, `legacy.report` as a plain PHP template,
and `žluťoučký.účtenka-😀` as a Unicode path. The chooser and Cmd+B opened the
expected files, preserving OpenIDEA's existing layout and shortcuts. With focus
autosave temporarily disabled in this disposable profile (idle autosave stayed
off), unsaved provider namespace changes opened the original package template,
and an unsaved config root change redirected `panel` to the alternate theme.
All 12 disk witnesses stayed original during these overlay checks. Restart
replaced server 73102 with 73461; after initialization, both overlays survived.
The first definition requested during startup did not navigate and needed retry;
this limitation is retained rather than claiming immediate restart readiness.
Problems displayed the old package name as `invalid_laravel_view` and cleared it
after normal Undo/save restored the namespace.

A new plain PHP template outside `views/` appeared in the already cached chooser
and opened at 1:1. External deletion while its editor was open removed the tab;
reopening and filtering the chooser produced no static matches. Chooser Cancel
left files unchanged. One batched tab-switch/Undo test action ran before focus
settled; repeating with confirmed editor focus restored the original config.
Normal save and explicit disk read-back verified all 12 originals. Focus autosave
was restored to true, idle autosave remained false, and Problems was hidden again.
Normal quit, no-argument relaunch (all ten tabs and original source), and final
quit passed. All owned app/server processes were absent afterwards. No existing
environment files were changed.

The development bundle executable SHA256 is
`38636f7677c0a5da337d99a4974d42df3bdde6cee1522cb833dbd1c1cef3f169`;
engine SHA256 is
`0ef92bcb1199ea7f054d2d0aa0fcd20ebcf08d575f311514d1438cb2ff8d531c`.
Root-only local signing and post-runtime deep/strict signature verification pass.
Installed 0.1.21 retains SHA256
`048064d8cc6beb3985f36a445a5524992624e0413c919c1faa61cbd27d49fa16`
and a valid signature. Build/source, SDK-gate and both native receipts are under
`artifacts/checks/view-navigation-0.1.48-*`. Full-source 127 provenance, licenses,
source checks and validation receipts remain in its engine archive.

Storage: only idle task-owned PHPantom compiler `.rlib`/`.rmeta` cache files
were additionally pruned after verifying the preserved 127 archive. The receipt
`view-navigation-engine-compiler-libraries-pruned.json` records all 934 paths,
hashes and measured space. Engine executables, apps, sources, archives, fixtures,
installed tools and earlier deliveries were preserved. The successful Tauri debug
build finished compilation in 2m12s and produced a 330.33 MiB bundle.

This accepts the narrow FEAT-032 view-root/package/freshness slice. Cross-file
view-data cycles, include/component boundaries, broader shadowing and native
in-flight project-switch cancellation remain open. No production signing,
notarization, installation, publishing, updater activation or further push.

Post-commit verification: the hook reformatted only the Tauri icon array among
runtime inputs. Serializing the committed JSON values with the build-time
indentation reproduces the frozen config SHA256 exactly, so configuration values
are identical. All runtime code/engine inputs match the frozen build; other
post-build differences are documentation. This is recorded explicitly in the
checkpoint receipt instead of claiming every source byte stayed identical.

### Next FEAT-032 slice: recursive view-data freshness (in progress)

After accepted 0.1.48 checkpoint a0351d7, add actual-client coverage for implicit
and explicit two-view cycles, unsaved external type changes, stale member
retirement, hover, engine restart, removing the only external seed and closing
the caller overlay. These synthetic cycles test static termination only and are
never rendered. No engine change or new build is claimed yet. Current signed
0.1.48 remains the accepted view-root/package delivery.

Recursive view-data checks against bundled 127 pass for both implicit and explicit
two-view cycles: external User/Other type retirement, completion, hover, mapped
definition, restart, removal/restoration of the external seed, and closing the
caller overlay. The expanded diagnostic-publication assertions pass21.26s: the
actual client receives unknown_variable after seed removal and its clear after
restoration/closing. Engine source and binary remain unchanged.

Native 0.1.48 cycle fixture (Laravel13.37,88 PHP syntax checks) verifies initial
userOnly:string and Original.php3:33 definition, unsaved controller Next type
with otherOnly:int replacing stale suggestions, Tab completion and mapped
Next.php3:26, both overlays surviving restart74893->76037, and missing-seed
diagnostics. The source witness confirms all5 originals while both controller
and leaf are unsaved. Native Enter-insert/Tab-replace and auto-insert-single
are intentional inherited defaults, not an integration fix. The initial
duplicate-name action was undone before saving; with auto-insert temporarily
disabled, the tooltip explicitly confirms the two modes.

Native failure retained: after restoring the external seed, completion returns
userOnly and Problems clears, but cached editor underline/error count persist.
No cycle batch acceptance. The root cause is the diagnostics view plugin's
constructor refreshing only nonempty diagnostic stores: EditorSlot caches lint
state while unmounted, so an empty list published while inactive never clears
the remounted state's old marks/count. A real EditorView regression reproduces
error count1 versus expected0 after remount. The two-line correction always
schedules current diagnostics for a path, including empty lists; focused tests
now4pass. Native corrected-build acceptance remains open.

Missing PHP type also exposes generic HTML snippet fallback; this separate
embedded-context limitation is open, not a no-suggestions native pass. All5
fixture originals were restored by normal Undo/Save All and read-back before
normal quit; all app/server generations exited. The owned synthetic profile
retains autoInsertSingle=false/focus autosave=false only for controlled next
comparison; restore true defaults after acceptance. Problems layout restored.
Receipt:view-data-cycles-0.1.48-native-diagnostic-result.json.

Frontend3049/type/lint/build passed before the remount correction; they must be
rerun on final runtime inputs. Repeated SDK workspace initially failed ENOSPC
while atomically rebuilding the 2.1GB static archive. Its log is preserved.
Only the idle regenerable old archive was removed with hash/space receipt; the
retry compiled successfully and is running workspace/client/host/fmt/Clippy
gates. Current49 identity/package work must wait for that pipeline to finish.

User approval (2026-10-31, asynchronous reply): OpenStorm's own process-launch
tests may create/delete only disposable fake .env fixtures containing FROM_FILE,
OVERRIDDEN and temporary PYTHONPATH values. This covers the two previously
filtered SDK cases, env_files_system_env_and_redirects and
python_scripts_import_sibling_packages_and_preserve_env_file_paths. Existing
environment files remain unchanged. The next final workspace gate will run
unfiltered; earlier filtered counts remain historical evidence.

Pre49 SDK supplement pipeline33298 finishes terminal0:1,319passed/44ignored/
2filtered, all19actual-client129.72s/all5host37.96s, fmt and Clippy pass.
Final49 gates must rerun unfiltered under the newly granted fake env-fixture
approval. For measured build headroom, only683 obsolete idle compiler rlib/rmeta/
static cache libraries were pruned, with per-file hashes and guards; changed or
empty/unrecognized candidates were preserved. All MH_OBJECT groups, actual apps,
executables, dylibs, sources, engines, fixtures, installed tools and earlier
deliveries remain untouched. Measured free space8,636,485,632bytes before49.
Receipt:view-data-cycles-sdk-obsolete-libraries-pruned.json. Root/34workspace
identities and app manifests are now0.1.49; engine/source pin remains127.

Final49frontend pipeline14786 finishes terminal0:3,050tests/352files, typecheck,
lint and production frontend build pass. Inherited jsdom performance notice
and Vite dynamic-import warning remain; no new warning suppression. SDK49
unfiltered pipeline50763is compiling with the newly approved process cases
included. Both app manifests and34lock identities agree0.1.49; no engine
resource/source changes. Native49bundle is not built or accepted yet.

Final49 unfiltered workspace tests pass:1,321passed/44ignored/0filtered. Both
newly approved process-launch environment cases positively report ok. Only
suite-owned fake fixtures were authorized; no existing env changes. The19live
client/five host, finalfmt/Clippy stages of pipeline50763are still running, so
this workspace result does not yet establish complete final49gates/native.

Final49 live-client suite passes19/19 in129.95s. The following host build fails
at openidea-server linking with ENOSPC before tests run; pipeline50763 exits101.
Workspace1321/44ignored/0filtered and live-client results remain valid; no
source change prompted a repeat. Frozen failure log/receipt retained.

Only idle old incremental-style workspace library compiler MH_OBJECT groups
predating frozen47 were pruned next:46,863groups/88,606hardlink paths, each
group header, inode, timestamp, complete hardlink set and SHA256 recorded before
deletion. The operation stopped at measured6,445,518,848free bytes. Direct app
objects, all nonincremental/current groups, apps/binaries/dylibs/libraries, engine
archives/sources/stubs originals, fixtures, installed tools and prior deliveries
remain intact. This supersedes the earlier action-specific statement that all
objects were preserved during library-only pruning. Receipt and append-only
event log:view-data-cycles-sdk-old-incremental-objects-pruned.{json,jsonl}.
80 copied generated stub-cache JSON files were also pruned only from idle
PHPantom compiler OUT_DIR; pinned source originals and binary unchanged.
Next: retry only failed host stage, then fmt/Clippy; package49 after success.

Final49 code gates complete: unfiltered workspace1,321/44ignored/0filtered,
frontend3,050, all19live-client129.95s, all5host35.01s after isolated link retry,
fmt and Clippy all-targets-Dwarnings (5m44s) pass. Host retry pipeline47755
terminal0. The initial host ENOSPC remains retained; no test-source changes
between attempts. code-gates.json records runtime input hashes. Tauri debug
packaging starts next, with measured9.7GiB free; no further cache pruning planned.

### Inactive-tab diagnostic refresh: 0.1.49 accepted within the tested scope

The frozen app executable SHA256 is
`056069eb452cd7db3bb868cbe3a986b0d456798a6020d3b042e65a4b2475e227`.
Engine 127 and its SHA256 remain unchanged. The Tauri debug app is 330.27 MiB;
root-only local signing and post-runtime deep/strict verification pass. Installed
0.1.21 and the archived 0.1.47/48 signatures also verify; installed 21 retains its
previous executable hash. No installation or push was performed.

The first native attempt (app 77081 / server 77099) exposed only a native frame
and blank captured main surface. Settings opened and responded. CUA reconnect,
app-name and identifier attempts did not restore a usable main window; the Mac
was verified unlocked. An explicit auxiliary launch created a separate owned
process. The first diagnostic app/server were stopped by exact verified PID with
SIGTERM after checking all five source originals; this was not a normal-quit
pass. The fresh comparison process (77596 / 77603) displayed the full editor and
a real screenshot. `view-data-cycles-0.1.49-first-window-diagnostic.json` retains
the initial failed-control attempt. Its cause remains open.

In the visible comparison process, unsaved controller removal of the external
render seed produced the leaf's `unknown_variable` / `unresolved_member_access`
and an error count of one. Problems confirmed the actual diagnostics. Switching
to the controller unmounted the leaf. Normal Undo restored its original render
call; Project Errors positively cleared while the leaf remained inactive. On
return, the leaf displayed No problems found, with no `$person` underline. Its
completion showed `userOnly: string`; Cmd+B opened Original.php at 3:33. All five
disk originals matched throughout the unsaved comparison and after Save All.
This is the direct corrected comparison for the retained 0.1.48 stale-state
failure, not acceptance inferred only from passing mocks.

Both temporary profile settings were restored through existing Settings:
auto-insert-single true and focus autosave true; idle autosave stayed false.
Problems was hidden again. Normal Cmd+Q stopped app 77596 / server 77603. A fresh
no-argument launch (78414 / 78424) restored the same Laravel project, all four
tabs, Original.php 3:33, original sources and No problems found. This normal
relaunch rendered correctly. Explicit Restart Language Servers replaced 78424
with 78752; the old process exited, engine 127 initialized, and the recursive leaf
again showed No problems found and Cmd+B resolved Original.php 3:33. Final normal
quit stopped app/server, and all owned 49 generations were absent. No UI reads
were issued after Cmd+Q that might accidentally relaunch a copy.

A quiet development-fixture sample recorded parent RSS 209,376 KiB and engine
193,680 KiB across 10.025 seconds; cumulative CPU increased 0.25s and 0.01s,
respectively. This excludes WebContent/GPU and is neither total application RAM
nor a parity benchmark. The first failed-window attempt logged first IPC 5,359ms;
that is not first usable UI. The structured idle, build, code-gate and native
receipts are under `artifacts/checks/view-data-cycles-0.1.49-*`.

The explicit and implicit cycles, current-type completion/hover/definition,
external-seed removal/restoration, restart, overlay closure, and actual diagnostic
publication/clear are also covered by the 19-case client run (129.95s); all five
host cases pass (35.01s on the isolated retry). The unfiltered workspace run is
1,321 passed / 44 ignored / zero filtered, including both newly approved fake
process environment cases. Frontend 3,050, type/lint/build, Rust fmt and Clippy
all-targets with warnings denied pass. Initial ENOSPC host-link failures and
compiler-cache recovery receipts remain retained; tests were not relabeled as
passed before their isolated retry succeeded. No original source/stub fixtures
or actual app/engine executables were pruned. Regenerate the copied compiler
OUT_DIR stub test caches before any future source-suite run that requires them.

This accepts the narrow FEAT-017/032 freshness correction and tested cycle slice.
It leaves the first-window anomaly, generic HTML fallback for missing PHP types,
broader view-data/component/shadowing combinations and the remaining 69-area
requirements open. New debugger expansion, production signing/notarization,
public release, update activation, real profile migration and deployment stay
deferred.

### Batch 0.1.50: component prop inference (narrow slice accepted)

This continues FEAT-032/033 after accepted checkpoint `76efd3e`. Required
anonymous-component `@props` bindings now preserve supplied model types. Prop
defaults fill missing or null values; supplied non-null values and explicit
PHPDoc contracts keep their types. The inherited `@aware` fallback is preserved;
parent-component consumable-data inference remains open. All 69 ledger rows
retain their prior partial/deferred status pending their full requirements.

**Reproduction and correction.** Engine127 receives the supplied User model in
a bare component, then loses its members when required `@props` is added.
`component-props-127-client-control.log` records that failure. Independent source
regressions also show `mixed` replacing the required model, and default `1`
overriding supplied `type="danger"`. Laravel13.37's `compileProps` uses supplied
values before null-coalescing defaults, confirming those inherited expectations
were wrong. The fix uses the existing PHP type representation and forward walker.

The development client probe passes completion, hover, mapped definition after a
same-line Unicode prefix, parent-variable isolation, unsaved caller changes,
nullable defaults, restart and closing the caller overlay (6.81s). All six disk
originals remain unchanged. The expanded Blade source run passes 365 tests,
including an eight-case model/default/null/false/PHPDoc matrix. Final client and
packaged acceptance against the production-feature rebuild are still required.

**Reproducible engine128.** Patch
`0128-blade-component-supplied-props.patch` has SHA-256
`7726cb71219d204b67a564b115c9aa93ff2aba40aabc935a7328458b62f1155b`.
The series identity is
`2fb9f6bcce2f7e7f84fa647cd3246921c25371a67cb60c5f8302daeba7e3c37c`.
After all-target Clippy fix/check and formatting, `prepare-phpantom.py` replayed
the patch into an independent verified tree matching the working source.
The preserved127 source still matches all 2,456 recorded files.

The reconstructed full source suite passes 18,388 tests, with 17 ignored and zero
filtered, plus the separate FIFO child. All-target Clippy reports no warnings;
formatting, source fingerprint and locked production-feature rebuild pass.
The archived executable SHA-256 is
`2f9a6249aa9105981be6bc57a7c8eb1778c74e0e3703c7d2f5bba6c725a4b3d8`.
Its provenance, licenses, build and validation receipts are under
`artifacts/engines/0.11.1-openstorm.128/`. The 12 plain PHP lints and runtime
assertions pass. Updated Laravel13.37 example analysis reports exactly its three
intentional errors at 731/731/951. Local50 resources now contain this engine.
Source gate and example evidence: `component-props-128-pinned-*` and
`component-props-128-examples-result.json`.

**OpenStorm gates.** Final50 frontend checks pass all 3,050 tests, type checking,
lint and build. The unfiltered workspace passes 1,321 tests, with 45 ignored and
zero filtered; both newly approved disposable environment-file process cases
positively pass. Existing environment files and profiles remain unchanged.
Rust formatting passes. Root Clippy and the 20 client / five host cases are still
running before packaging and native acceptance. The initial frontend run is
retained as49 baseline evidence: Python metadata preparation initially failed
before any mutation, was corrected, then final50 frontend checks passed.

**Native fixture.** `component-props-native-fixture.json` records an owned
Laravel13.37 copy with 61 PHP syntax checks, valid App PSR-4 mapping, no copied
environment files, two model types, an unsaved caller and component scope probes.
No fixture has been rendered. Native50 completion, definitions, restart,
reopen/relaunch and process cleanup remain pending. The signed49 app and
installed optimized21 app remain preserved; the earlier49 first-window anomaly
is still an open limitation, not a claimed fix.

**Build headroom.** SDK compilation left about1.9GiB free. Only the owned Python
coordinator was paused while its current Cargo child finished normally. With
compiler processes idle, 58,421 old pre-frozen47 incremental library MH_OBJECT
inode groups (63,988 hardlink paths; 7,268,999,456 logical bytes) were pruned with
per-inode metadata/header/SHA checks and append-only receipts. No source,
fixture, app, executable, dylib, library/archive, installed tool or prior delivery
was removed. Free space reached 9,664,192,512 bytes; the coordinator resumed and
no SDK stage failed. Deep/strict signatures of47/48/49 and installed21 still
verify, and installed21 plus engine127/128 checksums are unchanged. Evidence:
`component-props-sdk-old-incremental-objects-*`,
`component-props-0.1.50-sdk-stage-pause.json` and
`component-props-preserved-apps-after-cache-prune.json`.

Final50 code gates are complete:20actual-client cases pass (136.07s), and all
five host edit cases pass (35.21s), with shared fixture originals restored.
The component regression was strengthened to require the exact User property
selection after a same-line Unicode prefix; its focused rerun passes (6.82s),
followed by another formatting and full workspace/all-target Clippy pass.
The pre-packaging summary and runtime input hashes are recorded in
`component-props-0.1.50-code-gates.json`. Packaging and native acceptance remain
pending; no blanket feature-area promotion or public release is claimed.

**Packaged50 native evidence and open gate.** The arm64 development app is
`artifacts/batches/0.1.50/OpenStorm.app`, built in169.27s (328.82MiB) and locally
signed with deep/strict verification. Launch Services opened the owned fixture
with isolated backend settings. Required props offer `userOnly: string` and
Command-B reaches `Original.php`3:33 after the same-line emoji prefix. Unsaved
caller changes offer `otherOnly: int`; editor-focused Tab replaces the existing
member, and native undo/redo works. A nullable prop default preserves the model
and Command-B reaches `Next.php`3:26. Native restart retires39025 and starts39757,
retaining both unsaved overlays and their current inference.

Native Save All displays **File changed on disk** with Overwrite/Cancel. Cancel
preserves the external caller marker and unsaved Next buffer; the non-conflicting
component saves separately. Closing the dirty caller shows **Unsaved Changes**;
Don't Save closes its overlay, and fresh completion returns the disk Original
model's `userOnly: string`. Native undo/save restores the component. Only the
owned external marker is then removed from the closed caller. All five fixture
files match their originals exactly. Both temporary profile preferences are
restored to true (focus autosave and automatic single-result insertion).

Normal Command-Q exits app39016/server39757. A no-argument Launch Services
relaunch restores the project, three tabs and `Original.php`3:33. Default single
completion inserts `userOnly` into an empty member access; Save All and navigation
retain the original bytes and target. Focus autosave also persists a native
trailing-comment witness when Settings opens, without an explicit Save. That
owned witness is removed, all originals are verified, and another native
Command-Q exits app40455/server40466.

The last confirmation run is **not accepted**: app40934/server40945 initialize
and open the project, but CUA reports only a blank gray native frame. Raise and
activation attempts do not restore control, and a coordinate attempt reports
`noWindowsAvailable`; the Mac console is unlocked. First IPC is636ms, which is
not usable-UI proof. Earlier Settings closure also lost CUA control and required
stopping only owned38192/38568 with SIGTERM; that is not a normal-quit pass.
The source already clamps saved caret positions, so no speculative restore fix
was applied. A user observation of the actual window has been requested to
separate an app-rendering failure from native automation/window-state failure.
The last owned app/server remain running for that observation. Healthy idle
metrics and final cleanup are pending; no blanket startup/native gate pass or
feature-area promotion is claimed. The67partial/2deferred ledger is unchanged.

Evidence is consolidated in `component-props-0.1.50-native-result.json`, with
separate conflict, autosave, Settings-close and launch receipts. The props fix,
all code gates and the signed candidate are reviewable; final native acceptance
remains partial. No public publishing, new push, migration, update-feed activation,
notarization or debugger expansion was performed.

**Final native follow-up.** A normal macOS project-open event to the exact running
50 bundle recovered the full workbench without replacing app40934/server40945.
The original source, three tabs and correctly clamped caret4:1 were visible; the
inspector console showed no errors and was closed. This supersedes the pending
user-observation/cleanup status above. The earlier gray/control and Settings-close
failures remain recorded; no renderer or caret fix was justified, and general
first-window reliability remains unaccepted.

A ten-second healthy, warmed sample measured app RSS273,120KiB and server
RSS148,240KiB at the end, with CPU-time growth0.02s/0.00s. These two processes
exclude WebContent/GPU and do not establish total memory or startup parity.
Final native Command-Q stopped both40934/40945; no owned candidate app/server
remained. All five fixture hashes match the originals. Candidate50 and preserved
installed21 still pass deep/strict signature verification.

`component-props-0.1.50-native-final.json` consolidates this follow-up with the
prior workflow evidence. This accepts the bounded required/supplied-prop slice,
with the activation limitation above. The broader roadmap remains active.
Next: probe nested anonymous-component forwarding, caller/component scope
boundaries and unsaved dependency freshness through the actual OpenStorm client
before deciding whether another engine correction is needed.

### Nested component forwarding and named-slot scope: engine128 acceptance

The new actual-client regression
`laravel_nested_component_forwarding_tracks_unsaved_scope_boundaries` verifies
Controller → closed page → card → badge, including an aliased child prop. It
checks exact property navigation after a same-line Unicode prefix, unsaved
User→Other propagation across both component edges, isolation of an unbound
grandparent variable, restoration of the valid binding, engine restart and
closing the caller overlay. All seven disk originals remain unchanged. Named
slot bodies retain the caller's Other model, with completion, hover and exact
property navigation after the Unicode prefix. This is inherited engine128
behavior; no additional engine correction or runtime build was needed.

The first slot-navigation probe requested a definition immediately after a
change and returned no target; a diagnostic completion received cancellation.
Using the established settled-completion barrier before navigation produces the
correct current result. These preliminary logs are retained; they do not prove
an engine defect or general in-flight navigation readiness. The focused settled
run passes6.89s. The full actual-client corpus then passes21/21, zero filtered,
in143.69s. Evidence: `nested-components-slots-128-*` and
`nested-components-128-client-host-result.json`.

The same signed50 candidate also passed saved-source native navigation in a new
owned Laravel13.37 fixture: badge Command-L2:23 / Command-B opens User.php3:29,
and the named-slot Command-L1:79 / Command-B opens Other.php3:27. Both positions
follow a same-line emoji. The fixture passes61 PHP syntax checks and App PSR-4
validation; no environment files were copied. Normal quit, no-argument relaunch
with all four tabs and exact Other3:27 caret, and final normal quit pass. No
candidate app/server remains, and all six fixture originals match. Native
unsaved/restart/isolation acceptance is not newly claimed for this nested case;
those checks used the actual client. Receipt:
`nested-components-0.1.50-native-result.json`.

Rust follow-up passes1,321 workspace tests,46 ignored and zero filtered; both
approved disposable environment-file tests positively pass. Full workspace/all-
target Clippy with warnings denied, formatting and diff whitespace checks pass.
The consolidated receipt is `nested-components-128-final-gates.json`; all eight
frozen runtime-input hashes still match. Candidate50 and preserved installed21
retain deep/strict signature verification. The preceding50 frontend,
engine-source and host gates still describe the unchanged runtime; this follow-up
changes only regression coverage and this record. All69 areas remain visible at
67partial/2deferred. Parent-component `@aware` consumable-data inference, broader
component/slot combinations and the earlier native activation limitation remain
open. No new push, installation, publication or debugger expansion.

### Parent-component consumable data: FEAT-034 in progress

After accepted local checkpoint2e10faf, the next actual-client regression is
`laravel_aware_component_data_tracks_explicit_ancestors_and_unsaved_callers`.
Bundled128 passes the explicit-prop forwarding control, then fails after the
child changes to `@aware(['person'])` and explicit child forwarding is removed:
User members disappear. `component-aware-128-client-red.log` records the red
result. The test also specifies exact mapped navigation, unsaved User→Other,
restart and closing the caller overlay for the eventual corrected engine. This
new case is not accepted and has not been promoted into the packaged runtime.

The owned Laravel13.37 framework source provides the runtime contract:
`CompilesComponents::compileAware` calls `getConsumableComponentData`; that lookup
checks current rendered component data first, then still-open component-stack
data from nearest to outermost, using `array_key_exists`. Thus supplied null and
false remain supplied and do not select the default. Current rendered data can
win over a nearer still-open tag; blindly walking nearest HTML ancestors first
would be wrong. Anonymous `@props` defaults are local variables and must not be
promoted into consumable data. Six direct checks against the actual framework
trait pass in `component-aware-laravel13-runtime.log`; the owned probe creates
no environment files, boots no application and renders no fixture.

A separate mutable source copy starts from immutable verified128 at
`artifacts/phpantom/component-aware-129-work/`. The draft `blade/consumable.rs`
models presence independently of nullability and separates current-rendered
from open-tag data. Its initial focused source unit build is running; it is not
yet connected to inference, source reconstruction, provenance, production-feature
builds or the app. No engine129 binary, new app version or completed feature is
claimed. Preserve the immutable source, signed50 and installed21. Remaining work
includes context propagation through component and view edges, inline-tag and
caller correlation, missing/default/null handling, semantic refresh/restart,
class-backed/dynamic cases, focused and full gates, and packaged native checks.

Working129 follow-up: the initial seven model tests passed six cases and exposed
an unsafe `User|mixed` union in the unknown-data case. The draft now uses the
shared PHP type simplifier to retain `mixed` as the top type. Caller alternatives
also retain correlated current/open frames, so two callers that each supply a
key through different channels do not invent an absent-key/default branch.

The draft is now connected to cached Blade scopes, component and ordinary-view
edges, transparent cycle propagation, semantic scope comparison and lowering.
The prologue uses presence-based aware declarations and a virtual boolean marker
for genuinely optional fallback branches; supplied null does not coalesce. It
keeps explicit PHPDoc authority. Class-backed, unresolved and dynamic component
data currently degrades conservatively to unknown rather than asserting missing
keys or retaining an older parent's concrete type. This is not complete
class-backed-component acceptance. The first wired release check passes20.04s.
A 15-case public-hover matrix is compiling/running in session72582, recorded in
`component-aware-129-source-matrix.log`; its outcome is not yet known. The initial
model failure log is retained. No patch/provenance/root-resource promotion has
occurred. An identified performance follow-up is to reuse the selected component
attribute resolution for context and normal props, and avoid resolving unrelated
sibling tag attributes when only one descendant/ancestor path is needed.

The first wired source matrix passes all15 scenarios0.07s. Its draft server
(`62d32b1-dirty`, SHA256
`d6adf9b1fa8e1ff073d34504014d28ac639ef0b81272bc674b857dcfa722a14a`)
also passes the actual OpenStorm client case6.76s: inherited User members, exact
property definition after Unicode, unsaved Other replacement, restart replay,
closing the caller overlay and all seven disk originals. The checksummed source
snapshot and binary identity are preserved in
`component-aware-129-first-wired-draft.json` and its draft patch. This is a
development server result, not packaged acceptance or a released129 binary.

Follow-up source changes reuse one selected attribute-resolution pass for normal
props and consumables, filter out unrelated siblings, use sorted lookup metadata,
and resolve an aware key without cloning every data entry. Class-backed views
also retain unknown exposure instead of inferring absence from their tag bags.
Two tuple-adapter omissions were found by the release check and corrected; the
next release check passes19.65s. The initial failure is retained. The source
matrix now has17 cases, including cross-channel caller correlation and an
unresolved-tag error-recovery case; one separate class-backed exposure check
accepts conservative unknown or the correct Item type, and forbids claiming only
the default. Nine context-model cases cover nullability, precedence, uncertain
data, canonical alternatives and masked values. The broader Blade source
regression build is running in session63105 with
`component-aware-129-blade-regressions.log`; no outcome is claimed yet. Full
source/example checks, patch reconstruction/provenance, production-feature server
build, root/client/host gates, new packaging and native acceptance remain open.

Additional runtime boundary: `component-aware-laravel13-slots-runtime-buffered.log`
passes five checks using the real Laravel AnonymousComponent data and an owned
in-memory Htmlable probe. Anonymous initial data contains an attributes bag and
supplied null, but no automatic componentName. Still-open component data excludes
completed named/default slots; renderComponent then adds ComponentSlot values and
`__laravel_slots`, and restores the prior context afterwards. The first probe
incorrectly assumed slot(name, direct-content) wraps the content in ComponentSlot;
the retained initial log disproves that. Buffered slots use slot(name,null,attrs)
plus endSlot, as x-slot compilation does. Direct slot content retains its supplied
value. No application bootstrap, template fixture rendering or environment-file
mutation occurred.

This identifies a required follow-up before accepting the aware batch: distinguish
initial/open-tag data from rendered data in each component occurrence, include
known default/named-slot metadata only at render time, preserve named-slot owner
correlation, and keep unmodelled direct/dynamic slot writes conservative. The
current wired draft has not yet added that slot-metadata boundary. Broader Blade
regressions remain running in session63105; continue that same handle rather than
starting another build. The latest working source has no promoted patch,
provenance or app changes. Root changes remain the new ignored client case,
this progress record and narrowed evidence/remaining text for FEAT-032/033/034 in
the existing inventory. All69 feature IDs, full acceptance gates and status counts
remain unchanged (67partial/2deferred).

### Component-aware context: reconstructed129 findings and130 correction

The earlier running Blade check completed: 611 unit checks passed (1 ignored)
and 452 integration checks passed. The rendered-slot correction also passed the
actual OpenStorm client case in7.36s, including an unsaved named slot overriding
the attribute and closing that overlay restoring the original User type. These
results precede the stronger attribute-bag checks and do not establish the entire
batch's acceptance.

The staged-source recipe now accepts an explicit `--pin` in both preparation and
build scripts. Default and explicit128 reconstruction agree; malformed patch
checksums are refused, and the active source manifest and bundled128 binary are
unchanged. Source129 is frozen separately in
`engine/candidates/0.11.1-openstorm.129-source.json`, with patch129 and series
`6023af7414e42fcfff19c836ad1621dfe48d8b4b08f74c49b1ccfa9a10303a94`.
The reconstructed source matched all19 changed working files. Its preserved draft
server is `artifacts/engines/draft-aware-129-rendered-slots/phpantom_lsp`, SHA256
`677fdf10e343f9fa9e3e7d1c23c1a31d6d0b7ee9963a7259d134ceeb91e7ea3b`.
This is an unaccepted development binary, not the active app resource.

Full reconstructed129 source testing failed: 6937 unit checks passed (4 ignored,
plus the automatic FIFO child), then one analyzer-startup save check timed out.
The exact failing test passed alone and the complete five-test analyzer group
passed three subsequent controlled runs without source edits. The cause is still
unproven; neither those rechecks nor the Blade subset replace the failed full
gate. Details and original logs remain in
`artifacts/checks/component-aware-129-pinned-source-gates.json`.

Real Laravel13.37 isolated runtime checks establish that component attribute bags
can introduce consumable keys, explicit data overrides bag entries, raw Blade
attribute echoes are rewritten by Laravel's compiler, and numeric named slots
can produce integer keys. The corrected runtime probe passes four checks;
its initial incorrect assumption about PHP numeric-key renumbering is preserved.
No app bootstrap or existing environment-file mutation occurred.

Three stronger actual-client regressions fail against frozen129: unknown bag keys
incorrectly become absent/default-only; the slot map incorrectly promises only
string keys; ordinary frontend `:class` bindings shift PHP component bound-value
correlation after unsaved caller changes. The first bag test attempt had a test
index error after adding a stub; that is preserved separately and corrected using
a named source string. The subsequent failure is the actual engine result. Owned
bag-fixture PHP expressions and stubs pass PHP8.4.23 syntax checks.

Mutable130 corrects those causes: unknown bag/expression data retains uncertainty
while explicit fields keep priority; slot-map keys admit integers; only component
bindings advance PHP marker indices. The shared attribute lexer skips Blade
comments, echo bodies and directive arguments, including PHP arrows, while the
streaming preprocessor remains responsible for PHP/source mapping. New unit and
public-hover cases cover bag/null precedence, raw attribute expressions, numeric
slot keys and binding correlation. All-target Clippy with automatic fixes passes
with zero warnings; formatting was run afterwards. Host build-script stripping
is disabled only through the recorded build environment to avoid the inherited
rust-objcopy/libLLVM warning; no toolchain/global settings were changed.

Source130 is reconstructed from append-only patch130 and candidate source manifest,
series `56dc0837bb1fd98842e8d2591b8d1fe976ff66fd941fa4d78c88b5e936cf82d0`.
All six changed working files match reconstruction. Its full unfiltered source
pipeline is now running; results will be recorded in
`artifacts/checks/component-aware-130-pinned-source-gates.json`.
Current root actual-client inventory contains24 cases; the complete current
corpus has not yet passed130. No new app has been built or natively accepted.

The owned129 Laravel fixture was also corrected before acceptance: its inherited
lock file described13.24 while installed dependencies were13.37. The new fixture
now uses the coherent preserved123 exact13.37 manifest/lock; all75 installed
package versions and source/dist references match, all58 app PHP syntax checks
pass, and autoload roots/files exist. The older fixture and pre-correction files
remain preserved. No Composer execution or environment-file copying occurred.
See `artifacts/checks/component-aware-129-laravel-fixture.json`.

Storage maintenance removed only the remaining28,957 previously inventoried,
hash/header/hardlink-validated obsolete SDK compiler object groups (3.47GB logical)
while compilers were idle. Source, apps, engines, tools, fixtures and earlier
artifacts were preserved. Both development50 and installed21 retained their
executable hashes and deep/strict signatures afterwards. Receipt:
`artifacts/checks/component-aware-129-stale-sdk-objects-pruned.json`.
The full69-area status remains67 partial and2 user-deferred.

130 follow-up checks: all12 plain PHP files lint, the plain PHP runtime assertions
pass, and all267 Laravel assertions pass against the coherent owned13.37 fixture.
The expanded root actual-client corpus compiles with no warnings, and workspace
format checking passes. Bag overlay phases now require the preceding User type
to disappear (mixed or the new Other type is allowed), so stale User hover cannot
satisfy either bag phase. Closing the overlay must restore User members.

A fresh native fixture and isolated profile are prepared in
`artifacts/checks/component-aware-130-native-fixture.json`; all88 non-vendor,
non-Blade PHP files lint. It copies the coherent129 fixture while excluding every
`.env*`, project-local settings, Git metadata and node_modules, and adds owned
aware/card/badge/slot-map examples with distinct User/Other properties. No new
app is launched, and native acceptance has not started. Fixture preparation and
passing syntax checks do not establish editor behavior.

The full reconstructed130 source suite now passes in707.42s including compilation:
18,408 checks plus the separate FIFO child check,17 ignored, no top-level filtering.
The previously failing analyzer save/startup case passes inside the full five-test
group, and the new attribute-expression, bag-precedence and numeric-map regressions
pass. The original129 timeout remains preserved; no analyzer production code or
deadline was changed. The pipeline continues with all-target Clippy, fmt,
fingerprint verification and the production-feature recipe build. All3050 frontend
tests, type checking and lint also pass (version50 frontend, resource128 unchanged).
Build, actual-client/host and packaged-native acceptance are still pending.

130 production recipe build completes in203.08s. The archived arm64 server reports
`phpantom_lsp 0.11.1-openstorm.130`, SHA256
`7be1c87a374be3822eeefd8ca0c667df59baed75b3f5a3e59ad30103b902ba88`.
Its candidate binary provenance, complete source-gate receipt, licenses and
compiler/environment record are preserved in
`artifacts/engines/0.11.1-openstorm.130/`; the active app resource remains128.
Full static CLI analysis of the coherent Laravel fixture reports exactly the
three intentional diagnostics at731/731/951: one error and two warnings, with no
additional file diagnostics. The earlier wording “three errors” describes these
three diagnostics, not three error-severity findings. The actual24-case client
corpus and five host edit checks are running against the archived production
server. The three newly strengthened aware/bag, numeric-slot and frontend-binding
client regressions have passed; final corpus and native acceptance are pending.

### Development0.1.51 prepared: final SDK and native gates pending

All24 actual OpenStorm client checks pass against the archived production130
server in189.86s including the client rebuild; all5 host edit checks pass as
recorded in `component-aware-130-client-host.json`. The shared12-file fixture
remains byte-for-byte original after both stages. The full client corpus includes
the three new aware/bag, numeric-slot and frontend/directive correlation cases,
plus inherited completion/navigation/imports/rename, overlays and restart checks.
This is actual client/host evidence, not packaged UI acceptance.

Only after those gates passed, the source manifest and app resource were advanced
to130, checksum verified. Version0.1.51 is set consistently in the application,
workspace and34 local lock records. The earlier128 source, binary and signed50
bundle, unaccepted129 drafts and installed21 remain preserved. Root changes are
uncommitted; no new push, install, migration or publishing occurred.

The complete unfiltered0.1.51 workspace test/fmt/all-target Clippy pipeline is
running in session49738, with logs and stage results under
`component-aware-0.1.51-sdk-*`. It includes the two previously approved disposable
process environment cases; no existing environment-file mutation is authorized.
The final0.1.51 frontend build is running separately in session10569. The prior
3050 frontend tests/typecheck/lint cover the unchanged frontend code; the new
build checks the final versioned inputs. A signed native51 bundle and its UI
acceptance remain pending. The next step is to finish these same processes,
package the development app, and exercise the prepared isolated native fixture.

Final0.1.51 SDK gates pass:1321 workspace tests,49 ignored,0 filtered; workspace
formatting and all-target Clippy with warnings denied pass. Both approved
process-launch fake environment cases positively pass, and the bundled Tools row
reports the actual130 executable version. The final frontend build passes too.
See `component-aware-0.1.51-sdk-gates.json` and workspace summary.

The rebuild reduced free space to about1GB. The SDK orchestrator alone was
suspended before its next stage while its current workspace tests continued and
finished normally; no test process or deadline was suspended. Once the Cargo child
was terminal and compilers were idle, guarded cleanup removed32,222 obsolete
incremental-style workspace MH_OBJECT groups (37,015 paths), predating frozen48,
with complete hardlink/inode/size/mtime/header checks and SHA256 event records.
4,201,763,216 logical bytes were reclaimed; free space rose from1,083,150,336 to
5,369,999,360 bytes. All apps, source, current/direct compiler objects,
libraries/archives, engines, tools and fixtures were preserved. The same
orchestrator was resumed, collected the passing workspace exit, and passed its
remaining stages. The pause receipt confirms no resume is outstanding.
Receipts: `component-aware-0.1.51-stale-sdk-objects-pruned.json` and
`component-aware-0.1.51-sdk-stage-pause.json`. Installed21 and development50 retain
both their exact executable hashes and deep/strict signatures afterwards.

Development app packaging is now running in session64644 through
`build-component-aware-0.1.51-app.py`. Its guarded pipeline will copy to a unique
`artifacts/batches/0.1.51/OpenStorm.app`, verify bundled130/version/identity,
locally sign it and generate the source/build receipt only after a successful
build. No51 bundle or native acceptance is claimed until those steps complete.
Continue this same build handle; the intended isolated native fixture is recorded
in `component-aware-130-native-fixture.json`. The69-area ledger remains unchanged
in status.

Development0.1.51 packaging finishes successfully in156.84s. The unique bundle is
`artifacts/batches/0.1.51/OpenStorm.app`, locally signed and deep/strict verified;
its build/source receipt is `component-aware-0.1.51-build-receipt.json`. Installed21
and signed50 remain separate. Native51 acceptance has started, not completed.

Observed native51 passes: healthy first project launch; Go to File discovers the
aware component; CmdL2:23/CmdB after the emoji maps to exact User.php3:29; empty
receiver CtrlSpace uses inherited automatic single insertion; explicit save returns
the exact original and clears its temporary diagnostic. Settings search reveals
default idle autosave off/focus autosave on. Focus autosave was temporarily disabled
through Apply only in the isolated profile, then normal CmdQ exited app55762 and
server56080. No-argument relaunch restores the same project, two tabs and2:30 caret.

In app56115, unsaved Caller User->Other changes the aware badge's completion to
otherOnly and definition to Other.php3:27, while all7 disk originals remain exact.
The native Restart phpantom action retains the unsaved caller model. The current
resource server56139 started later than the same app process and runs with
`--stdio --no-project-tools`. A further unsaved Other property appears in the native
lookup; Tab replaces the complete badge identifier with otherLong and definition
opens the unsaved Other.php3:53. Native Undo/Redo restores/reapplies the caller;
final undo and explicit saves restore all7 disk originals and User.php3:29 with
No problems found.

One initial native test expectation was wrong, not an established product defect:
CtrlSpace automatic insertion from inside an identifier produced
otherOnlytherOnly. The existing completion tests explicitly preserve suffixes for
insert/Enter and replace the rest with Tab. The native lookup also says Enter
inserts/Tab replaces. Native Tab replacement with two candidates passes exactly;
Undo restored the initial text. Both the observed insertion and corrected
expectation remain in `component-aware-0.1.51-native-progress.json`. No source
behavior was changed to override OpenIDEA's inherited completion convention.

Current native state: app56115/server56139 remain intentionally live; User.php3:29
is active; badge, Caller, Other and User tabs are open; no dirty overlay remains;
all7 owned fixture originals match. Focus autosave remains temporarily disabled
only in the owned test profile's `Library/Application Support/cz.thinky.openstorm/
settings.toml` and must be restored before final acceptance. Continue this exact
native session. Named-slot/bag/frontend-binding native cases, external-conflict
cancellation, restored default autosave, final reopen/relaunch and normal process
cleanup are still pending. Earlier50 window activation/control limitations remain
recorded; these healthy51 launches alone do not prove their underlying cause fixed.

### Native51 completion and focus-autosave correction for0.1.52

Native51 additionally passes the component edge cases against real Laravel13.37:
unsaved named person slot -> toHtml() completion and ComponentSlot.php56:21;
unknown attribute bag -> mixed, explicit User beside Other bag -> User, raw echo
then -> mixed; frontend :class before component -> fresh Other, @class argument
then caller undo -> User; raw ->merge/=> expression before explicit data -> fresh
Other; numeric named slot through an ordinary include -> exact
array<array-key,mixed> documentation. All7 disk originals were verified before
cleanup. CmdS follows inherited save-all behavior; its intermediate persisted
Caller/page changes were observed and all originals subsequently restored through
native edits/save.

Native51 external-save protection passes: dirty Other caller, externally written
User+witness -> overwrite confirmation; Cancel preserves both; retry prompts again;
explicit Don't Save retains the external bytes; native reopening reads User plus
witness; native cleanup and ShiftTab restore the exact original indentation and
all7 originals. Receipt: `component-aware-0.1.51-external-conflict.json`.

Focus autosave was restored to true in the owned profile, with idle-save still
false. Noargs relaunch restores8tabs and Caller6:1 with the73-character/two-line
selection. One post-quit AX observation reopened the candidate under ordinary HOME;
the guarded launcher refused before starting another instance. No preferences
were changed in that unexpected process. Bare CmdQ followed only by12 CLI process
observations proves all its app/server processes exited; further exit checks avoid
post-quit CUA reads. The subsequent explicit-profile launch verifies HOME before
UI inspection. See `component-aware-0.1.51-post-quit-observation.json`.

An actual remaining native failure blocks acceptance of51: User comment plus
Modified marker did not persist after CmdTab or Finder activation, despite the
owned profile's autoSave=true. Switching editor tabs persists it, proving the
setting and existing save path work. A controlled comparison in preserved51 also
fails with a newly created real Finder Recents window, not just the Desktop; Undo
plus tab-switch restores all7 originals, the temporary Finder window closes and
normal bare CmdQ exits all51 processes. Receipts: focus-autosave and
`native-focus-51-real-window-control.json`. These are observed failures, not a
passing autosave claim.

The editor subscribed only to DOM window blur. Four regression cases now exercise
native focus loss without that DOM event, manual-save preservation, automatic
external-conflict refusal with no prompt/overwrite, and delayed listener cleanup
after stopping. The initial three tests fail before correction; the existing
controller now also subscribes to Tauri's current-window onFocusChanged and feeds
loss into the same guarded auto-save path. Disposal and registration errors are
handled; native52 must still verify this hypothesis on the actual Mac. No save
conflict, settings, keymap, layout or engine behavior was bypassed.

Candidate52 uses unchanged engine130 bytes/provenance. All27 focused controller,
tab-close and document checks pass, as do all3054 frontend tests/typecheck/lint/build.
Full unfiltered Rust workspace/fmt/Clippy are running in session64504. Engine130's
previous full source/client/host checks remain tied to its unchanged checksum.

Before the rebuild, while compilers were idle, guarded cleanup reclaimed
7,455,617,144 logical bytes from5,800 obsolete standalone workspace MH_OBJECT groups
predating frozen48. Inodes, all hardlinks, sizes/mtime, object headers and SHA256
were checked and recorded. All current objects, direct app objects, libraries,
archives, apps, engines, source, fixtures and prior deliveries were preserved.
Free space rose from2,246,467,584 to9,664,106,496 bytes. Receipt:
`native-focus-0.1.52-old-workspace-objects-pruned.json`.

The user's latest instruction is to finish this batch and then commit/push. Do not
begin another roadmap batch first. Native52 still needs positive app-focus
autosave, guarded conflict behavior under that native event, inherited engine
smoke checks, restored fixtures/defaults, noarg session recovery, strict signatures
and final normal process cleanup before committing and verifying the pushed ref.

52 final source gates pass:1321 workspace tests/49ignored/0filtered, fmt/all-target
Clippy,3054 frontend tests/typecheck/lint/build. The signed52 development bundle
builds in95.83s and passes deep/strict signing; current controller/regression bytes
match its build receipt. Installed21 and signed50/51 hashes/signatures are also
preserved after cache cleanup. Remote origin fetch shows25 local commits ahead,
0 incoming; no push has yet occurred.

Native focus verification needs a clarified boundary. App-targeted Finder keys,
AX Raise and a title-bar click still did not persist the52 witness. They have not
independently established that macOS application activation changed. Opening the
native Web Inspector immediately saves it through the guarded blur path; its
console is empty. The preserved51 control's failed attempted switches likewise
must not be treated as a proven OS-focus defect. Thus the native current-window
listener has regression coverage, but its actual external-app focus path remains
unverified. No further speculative patch was stacked on this hypothesis.

The inspector is closed, the previous witness was undone/saved, and a fresh
unsaved `// actual-focus-switch-witness` is visible in the owned User fixture.
App52 PID6270 remains live under the verified isolated HOME. A single user action
has been requested: click the running OpenStorm, then Finder in the Dock, and
report completion. Keep this prepared UI state while awaiting that real activation;
do not silently substitute a tab switch or inspector opening for an external-app
switch. After verification, restore all7 originals/defaults, close the temporary
Finder window, finish the52 smoke/relaunch/process checks and commit/push as asked.

### Discarded conflicted documents across project replacement: development53

The prepared actual-focus-switch witness subsequently persisted exactly on disk,
with no automated Save or tab change between preparation and observation. The
native User tab no longer had a Modified marker. Undo/Save restored all7 originals.
The user later confirmed they are not at the Mac; therefore no physical Dock
switch is claimed. Preserve the actual persistence observation and the separate
unverified activation attribution in `native-focus-0.1.52-native-progress.json`.

Native52 also proves guarded automatic saving: dirty local User comment plus an
external witness, then the observed webview-blur path -> Autosave paused notices,
no overwrite prompt and no external write. Native File > Save All then asks before
overwrite; Cancel retains both versions. Inspector close left the automation
without editor keyboard focus, so native menus were used; no source fix for that
tooling/window-control limitation was claimed.

The subsequent native Close Project > Don't Save and recent-project reopen
exposed a real failure: discarded local text reappeared as Modified, while the
external file stayed intact. The old server exited correctly. Session storage
contains only view state, so this was not recovered text in serialized sessions:
DocumentManager retained its conflicted model by path after the host replaced all
project documents, then reused that stale model on reopen. The initial regression
fails with discarded text instead of the external version; its log is retained in
`project-discard-52-red.log`. After normal52 quit, the owned external witness was
archived and all7 originals restored; no user environment file was touched.

The correction resets frontend document identity when the host replaces the
project or LightEdit context. Old models are disposed, cache/meta state cleared
before close-event callbacks, and late old-project opens are cancelled without
removing a new project's pending open. A late release cannot remove a fresh model.
No new settings or close-consent behavior was added. Three regression cases cover
the native reproduction, overlapping old/new opens, and a close-event subscriber
acquiring the fresh model. All30 focused controller/document/close checks pass.

The first full53 frontend run passed3056 and failed only its required source-text
translation catalog for the new Project changed cancellation message. Adding the
Chinese catalog entry fixes that gate; final full3057/typecheck/lint/build pass.
Full unfiltered Rust workspace/fmt/all-target Clippy pass. Engine130 is unchanged,
so its checksummed source/client/host receipts remain applicable rather than
rebuilding an unchanged engine. Signed53 packaging is running through
`build-project-discard-0.1.53-app.py`; its returned process handle is authoritative. Its native discard/reopen check and final cleanup
remain pending before the requested commit and push.

Before53 rebuild, with compilers idle, guarded cleanup reclaimed6,639,908,672
logical bytes from5,188 obsolete standalone workspace compiler object groups
predating frozen48, with metadata/header/hardlink/SHA verification. Current objects,
all app binaries, libraries/archives, source, engines, fixtures, tools and earlier
deliveries were preserved. Free space rose3,010,215,936->9,663,889,408 bytes; receipt
`project-discard-0.1.53-old-workspace-objects-pruned.json`.

### Final development0.1.53 checkpoint and requested Git delivery

Signed53 packaging succeeds in96.74s. Executable SHA256:
`8b816d20c644f65cc3686772fea353c0b35e860e3300b5c3916ad06913969a00`.
The build receipt records the source snapshot; final controller, document manager,
regression, locale, engine pin and provenance bytes match that snapshot. The app
and bundled engine checksum pass strict/deep signing and provenance checks.

The native reproduction now passes in the packaged53 app with default autosave
on: local User comment + external witness -> tab-switch auto-save pauses without
writing; Close Project asks about the modified file; explicit Don't Save reaches
Welcome53 and preserves the external bytes. Reopening from native Recent Projects
and selecting User loads the external comment as a clean document, with no
resurrected local text. The original3-line User fixture is restored through native
save and all7 owned files match exactly. After this project replacement, aware
badge CmdL2:23/CmdB still resolves after Unicode to exact User.php3:29.

Normal CmdQ exits app54718/server55495. Explicit-profile noargs launch55527
restores the same project, all8tabs, active clean User.php and caret3:29, with No
problems found. Final bare CmdQ leaves no owned53 app/server process. The isolated
profile retains default focus autosave=true, idle-save=false; the temporary Finder
window is closed. Receipt: `project-discard-0.1.53-native-progress.json`.

A10.022s warm idle sample after project reopen observes app RSS203856->203760KiB,
CPU4.06->4.36s, and server RSS216080KiB, CPU5.61->5.61s. This excludes WebContent,
GPU and other processes, uses a warmed fixture/profile and is not a total-memory,
fresh-startup or parity result. Broader performance gates remain open.

Physical app-switch attribution is explicitly limited. The prepared
actual-focus-switch witness eventually persisted exactly without any further
Save/tab action from automation, and native UI showed it clean. No user completion
reply confirmed a Dock switch; the user said they were not at the Mac. App-targeted
keys/Raise/clicks cannot establish OS activation on their own. Keep that manual
case unverified. Guarded automatic persistence, tab/webview blur refusal, native
manual overwrite Cancel, model lifecycle regressions and clean relaunch/exit have
separate positive evidence. Inspector/window-control automation limitations remain
recorded; this checkpoint makes no unsupported claim that their cause is fixed.

Final review against2e10fafe941d1e589ca3b59af6668184aebf2d84 covers reproducible
source/binary provenance, shared typing, unknown-key conservatism, current-window
save guards, project identity/late-open cancellation, event subscriber ordering,
manual-save/conflict boundaries, unchanged debugger scope and preserved defaults.
The final default and explicit130 source reconstruction agree. All69 IDs and full
acceptance requirements remain visible. This is a local development checkpoint;
installed-app migration, publishing a release, updates, notarization and wider
physical/service matrices remain deferred or unverified as already recorded.

The user's instruction to finish this batch and commit/push is now carried out
without starting another feature batch first. The final Git commit/remote read-back
and clean-worktree evidence are stored in the ignored local delivery receipt;
repository history is authoritative for the pushed source identity.
