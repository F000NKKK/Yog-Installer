import { useEffect, useState } from "react";
import { api, type PlanView, type Selection, type Why } from "../api";

/** How long the plan waits for more clicks before it's asked again. */
const REPLAN_DELAY_MS = 150;

const without = (list: string[], id: string) => list.filter((x) => x !== id);
const withId = (list: string[], id: string) => (list.includes(id) ? list : [...list, id]);

/** The Modify window's checkboxes for `product`, and what they'd do. */
export function useModify(product: string) {
  const [selection, setSelection] = useState<Selection | null>(null);
  const [plan, setPlan] = useState<PlanView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.selectionOf(product).then(setSelection, (e) => setError(String(e)));
  }, [product]);

  useEffect(() => {
    if (!selection) return;
    const timer = setTimeout(() => {
      api.plan(product, selection).then(
        (p) => {
          setPlan(p);
          setError(null);
        },
        (e) => setError(String(e)),
      );
    }, REPLAN_DELAY_MS);
    return () => clearTimeout(timer);
  }, [product, selection]);

  const toggleWorkload = (id: string) =>
    setSelection((s) => s && { ...s, workloads: s.workloads.includes(id) ? without(s.workloads, id) : [...s.workloads, id] });

  /** Checks or clears component `id`; `why` is why it's in now (a required one can't be cleared). */
  const setComponent = (id: string, on: boolean, why: Why | undefined) =>
    setSelection((s) => {
      if (!s) return s;
      if (on) return { ...s, checked: withId(s.checked, id), unchecked: without(s.unchecked, id) };
      if (why?.kind === "recommended") return { ...s, unchecked: withId(s.unchecked, id) };
      return { ...s, checked: without(s.checked, id) };
    });

  return { selection, plan, error, toggleWorkload, setComponent };
}

/** Whether a component in for `why` can be cleared by hand. */
export function clearable(why: Why | undefined): boolean {
  return why === undefined || why.kind === "chosen" || why.kind === "recommended";
}
