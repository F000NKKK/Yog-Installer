import { useEffect, useRef, useState } from "react";
import { api, type ProgressEvent } from "../api";

/** A running (or just ended) operation, as a product card shows it. */
export interface Operation {
  product: string;
  packages: number;
  /** Bytes to download, as far as known, and done so far. */
  bytes: number;
  downloaded: number;
  /** Bytes a second, lately. */
  speed: number;
  /** The package installing: its place in the plan and its id. */
  installing: { index: number; of: number; component: string } | null;
  lastLine: string | null;
  paused: boolean;
  finished: { ok: boolean; cancelled: boolean; error: string | null } | null;
}

/** How long a download speed is measured over. */
const SPEED_WINDOW_MS = 1000;

function started(product: string, packages: number, bytes: number): Operation {
  return { product, packages, bytes, downloaded: 0, speed: 0, installing: null, lastLine: null, paused: false, finished: null };
}

/** The download so far, measured outside the state (once per event). */
interface Downloaded {
  total: number;
  speed: number | null;
}

/** `prev` after `event` (pure: React may call it twice). */
function reduce(prev: Operation | null, product: string, event: ProgressEvent, downloaded: Downloaded): Operation | null {
  switch (event.kind) {
    case "started":
      return started(product, event.packages, event.bytes);
    case "downloading": {
      const base = prev ?? started(product, 0, 0);
      return { ...base, downloaded: downloaded.total, speed: downloaded.speed ?? base.speed, bytes: Math.max(base.bytes, downloaded.total) };
    }
    case "installing":
      return prev && { ...prev, installing: { index: event.index, of: event.of, component: event.component } };
    case "output":
      return prev && { ...prev, lastLine: event.line };
    case "finished":
      return prev && { ...prev, finished: { ok: event.ok, cancelled: event.cancelled, error: event.error } };
    default:
      return prev;
  }
}

/**
 * The running operation, from the engine's progress events; `onFinished`
 * when it ends (the window re-reads what's installed then).
 */
export function useOperation(onFinished: () => void) {
  const [op, setOp] = useState<Operation | null>(null);
  // Each component's bytes so far (a download reports its own running total).
  const perComponent = useRef(new Map<string, number>());
  const sample = useRef({ at: 0, bytes: 0 });
  const finished = useRef(onFinished);
  finished.current = onFinished;

  useEffect(() => {
    const unlisten = api.onProgress((product, event) => {
      const now = performance.now();
      if (event.kind === "started") {
        perComponent.current.clear();
        sample.current = { at: now, bytes: 0 };
      }
      if (event.kind === "downloading") perComponent.current.set(event.component, event.done);
      const total = [...perComponent.current.values()].reduce((a, b) => a + b, 0);
      let speed: number | null = null;
      if (event.kind === "downloading" && now - sample.current.at >= SPEED_WINDOW_MS) {
        speed = ((total - sample.current.bytes) * 1000) / (now - sample.current.at);
        sample.current = { at: now, bytes: total };
      }
      setOp((prev) => reduce(prev, product, event, { total, speed }));
      if (event.kind === "finished") finished.current();
    });
    return () => {
      void unlisten.then((u) => u());
    };
  }, []);

  return {
    op,
    running: op !== null && op.finished === null,
    pause: (paused: boolean) => {
      void api.pause(paused);
      setOp((prev) => prev && { ...prev, paused });
    },
    cancel: () => void api.cancel(),
    dismiss: () => setOp(null),
  };
}
