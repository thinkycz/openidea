import { useState } from "react";
import { Dialog, Input, List, T, listNavigate } from "@/ui";
import { closeLaravelNavigation, selectLaravelItem, useLaravelNavigation } from "./laravel";
import styles from "./LaravelNavigation.module.css";

export function LaravelNavigation() {
  const state = useLaravelNavigation();
  return state.kind ? <LaravelNavigationDialog key={`${state.kind}:${state.phase}:${state.selectedName}`} /> : null;
}

function LaravelNavigationDialog() {
  const { kind, phase, selectedName, items, loading, incomplete, error } = useLaravelNavigation();
  const [filter, setFilter] = useState("");
  const [active, setActive] = useState(0);
  const visible = items.filter((item) => item.label.toLowerCase().includes(filter.trim().toLowerCase()));
  const current = Math.min(active, visible.length - 1);
  const pick = () => {
    if (visible[current]) void selectLaravelItem(visible[current]);
  };
  return (
    <Dialog
      open
      title={kind === "routes" ? "Laravel Routes" : "Laravel Views"}
      width={640}
      onClose={closeLaravelNavigation}
      buttons={[
        {
          label: phase === "names" ? "Show Targets" : "Open",
          role: "default",
          disabled: loading || !visible[current],
          onClick: pick,
        },
        { label: "Cancel", role: "cancel" },
      ]}
    >
      <div className={styles.content}>
        <div className={styles.hint}>
          <T>Static source index; dynamic registrations may be unavailable.</T>
        </div>
        {selectedName && <div>{selectedName}</div>}
        {incomplete && (
          <div role="status" className={styles.hint}>
            <T>Results are incomplete: dynamic names or additional matches may exist.</T>
          </div>
        )}
        {loading && (
          <div role="status">
            <T>Loading…</T>
          </div>
        )}
        {error && (
          <div role="alert" className={styles.error}>
            {error}
          </div>
        )}
        <Input
          data-autofocus
          aria-label="Filter Laravel results"
          value={filter}
          onChange={(event) => {
            setFilter(event.target.value);
            setActive(0);
          }}
          onKeyDown={(event) => {
            const next = listNavigate(event.key, { items: visible, active: current, wrap: true });
            if (next !== null) {
              event.preventDefault();
              setActive(next);
            }
          }}
        />
        <List
          items={visible}
          getKey={(item, index) => `${index}:${item.label}`}
          activeIndex={current}
          onActiveChange={setActive}
          onActivate={(item) => void selectLaravelItem(item)}
          focusable={false}
          aria-label="Laravel results"
          className={styles.list}
          empty={loading ? <T>Loading…</T> : <T>No static matches found</T>}
          renderItem={(item) => <div className={styles.row}>{item.label}</div>}
        />
      </div>
    </Dialog>
  );
}
