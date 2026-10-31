/** Shared reviewed edits stay one history entry even across closed files. */
import {
  StateEffect,
  Facet,
  Prec,
  Transaction,
  EditorState,
  EditorSelection,
  ChangeSet,
  type ChangeSpec,
  type Extension,
} from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { diff } from "@codemirror/merge";
import {
  invertedEffects,
  isolateHistory,
  undo,
  redo,
  undoSelection,
  redoSelection,
  historyKeymap,
} from "@codemirror/commands";
import { remoteChange } from "@/editor/api";
import type { ReviewedEditHistory } from "@/bindings";

let applyingReviewed = false;
const groupEffect = StateEffect.define<string>();
type Restore = (group: string, redo: boolean) => Promise<void>;
const handler = Facet.define<(group: string, redo: boolean) => void>();
type HistoryTarget = Parameters<typeof undo>[0];
function runHistory(target: HistoryTarget, forward: boolean, selection = false): boolean {
  let candidate: Transaction | undefined;
  const command = selection ? (forward ? redoSelection : undoSelection) : forward ? redo : undo;
  const found = command({
    state: target.state,
    dispatch: (tr) => {
      candidate = tr;
    },
  });
  if (!found || !candidate) return false;
  const group = candidate.effects.find((e) => e.is(groupEffect))?.value;
  if (group) {
    target.state.facet(handler)[0]?.(group, forward);
    // Missing handlers must never degrade to partial local undo.
    return true;
  }
  target.dispatch(candidate);
  return true;
}
export const reviewedUndo = (target: HistoryTarget) => runHistory(target, false);
export const reviewedRedo = (target: HistoryTarget) => runHistory(target, true);

const reviewedUndoSelection = (target: HistoryTarget) => runHistory(target, false, true);
const reviewedRedoSelection = (target: HistoryTarget) => runHistory(target, true, true);

export const reviewedHistoryKeymap = historyKeymap.map((b) => ({
  ...b,
  run:
    b.run === undo
      ? reviewedUndo
      : b.run === redo
        ? reviewedRedo
        : b.run === undoSelection
          ? reviewedUndoSelection
          : b.run === redoSelection
            ? reviewedRedoSelection
            : b.run,
  shift:
    b.shift === redo
      ? reviewedRedo
      : b.shift === undo
        ? reviewedUndo
        : b.shift === undoSelection
          ? reviewedUndoSelection
          : b.shift === redoSelection
            ? reviewedRedoSelection
            : b.shift,
}));

/** Keep default shortcuts and native Edit-menu history input, while retaining
 * the original history until the guarded backend commit arrives. */
export function reviewedHistory(restore: Restore, error: (e: unknown) => void): Extension {
  let pending = false;
  return [
    handler.of((group, forward) => {
      if (pending) return;
      pending = true;
      void restore(group, forward)
        .catch(error)
        .finally(() => {
          pending = false;
        });
    }),
    EditorState.transactionExtender.of(() => (applyingReviewed ? { annotations: remoteChange.of(true) } : null)),
    invertedEffects.of((tr) => tr.effects.filter((e) => e.is(groupEffect))),
    Prec.highest(
      EditorView.domEventHandlers({
        beforeinput(e, view) {
          const command =
            e.inputType === "historyUndo" ? reviewedUndo : e.inputType === "historyRedo" ? reviewedRedo : null;
          if (!command) return false;
          e.preventDefault();
          command(view);
          return true;
        },
      }),
    ),
  ];
}

/** Compact replacements can cover unchanged selected text. Refine only when
 * default mapping would invert a range, keeping work bounded for large edits. */
function reviewedChanges(state: EditorState, changes: ChangeSet) {
  const mapped = state.selection.ranges.map((range) => range.map(changes));
  const affected = state.selection.ranges.filter((_, i) => mapped[i]!.from > mapped[i]!.to);
  if (!affected.length) return { changes };
  const deadline = performance.now() + 10;
  const specs: ChangeSpec[] = [];
  changes.iterChanges((from, to, _fromB, _toB, inserted) => {
    if (
      to - from + inserted.length > 65_536 ||
      performance.now() >= deadline ||
      !affected.some((range) => from <= range.from && to >= range.to)
    ) {
      specs.push({ from, to, insert: inserted });
      return;
    }
    const before = state.doc.sliceString(from, to);
    const after = inserted.toString();
    for (const part of diff(before, after, { scanLimit: 500, timeout: Math.max(1, deadline - performance.now()) })) {
      specs.push({ from: from + part.fromA, to: from + part.toA, insert: after.slice(part.fromB, part.toB) });
    }
  });
  const refined = ChangeSet.of(specs, state.doc.length);
  const selection = EditorSelection.create(
    state.selection.ranges.map((range) => {
      const next = range.map(refined);
      return next.from <= next.to ? next : EditorSelection.cursor(refined.mapPos(range.head, 1));
    }),
    state.selection.mainIndex,
  );
  return { changes: refined, selection };
}

/** Consume the matching CM undo/redo entry instead of adding a second history
 * entry for its backend patch. Split panes without that entry just mirror it. */
export function reviewedTransaction(
  state: EditorState,
  changes: ChangeSet,
  history: ReviewedEditHistory,
  primary: boolean,
): Transaction {
  if (history.action !== "apply") {
    let candidate: Transaction | undefined;
    // Preserve CodeMirror's own history transaction and its private annotations;
    // mark it remote while constructing it so it never echoes as a local edit.
    const command = history.action === "undo" ? undo : redo;
    applyingReviewed = true;
    try {
      command({
        state,
        dispatch: (tr) => {
          candidate = tr;
        },
      });
    } finally {
      applyingReviewed = false;
    }
    if (
      candidate?.effects.some((e) => e.is(groupEffect) && e.value === history.group) &&
      candidate.newDoc.eq(changes.apply(state.doc))
    ) {
      return candidate;
    }
    return state.update({
      ...reviewedChanges(state, changes),
      annotations: [remoteChange.of(true), Transaction.addToHistory.of(false)],
    });
  }
  return state.update({
    ...reviewedChanges(state, changes),
    effects: primary ? groupEffect.of(history.group) : [],
    annotations: [remoteChange.of(true), Transaction.addToHistory.of(primary), isolateHistory.of("full")],
  });
}
