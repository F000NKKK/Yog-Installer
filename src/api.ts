import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeCommand } from "substrate-platform-ui";
import { events, installer } from "./host.generated";

/** `overview::ProductView` — a product card. */
export interface Product {
  id: string;
  name: string;
  description: string;
  installed: string | null;
  latest: string | null;
  updateAvailable: boolean;
  notesUrl: string | null;
  program: string | null;
  size: number | null;
  /** Its icon's URL. */
  icon: string | null;
}

/** `overview::ComponentView` — a component's line. */
export interface Component {
  id: string;
  name: string;
  description: string;
  group: string;
  installed: string | null;
  managed: boolean;
  latest: string | null;
  size: number | null;
}

export interface Group {
  id: string;
  name: string;
}

export interface Workload {
  id: string;
  name: string;
  description: string;
  icon: string | null;
  required: string[];
  recommended: string[];
  optional: string[];
}

/** `latest::News` — a release in the developer news. */
export interface News {
  of: string;
  version: string;
  title: string | null;
  published: string | null;
  url: string | null;
}

/** `pending::Pending` — an install or modify that didn't end. */
export interface Pending {
  product: string;
  selection: Selection;
  mode: Mode;
  keepDownloads: boolean;
}

/** `commands::Snapshot` — everything the window shows. */
export interface Snapshot {
  version: string;
  home: string;
  cache: string;
  products: Product[];
  components: Component[];
  groups: Group[];
  workloads: Workload[];
  productWorkloads: [string, string[]][];
  installerUpdate: string | null;
  pending: Pending | null;
  news: News[];
  problems: string[];
}

/** `selection::Selection` — the Modify window's checkboxes. */
export interface Selection {
  workloads: string[];
  unchecked: string[];
  checked: string[];
}

export type Change =
  | { kind: "install"; id: string; version: string }
  | { kind: "update"; id: string; from: string; to: string }
  | { kind: "remove"; id: string; version: string };

export interface Conflict {
  component: string;
  version: string;
  needs: string;
  requirement: string;
  found: string | null;
}

/** `selection::Why` — why a component is in a selection. */
export type Why =
  | { kind: "required"; workload: string }
  | { kind: "recommended"; workload: string }
  | { kind: "chosen" }
  | { kind: "requiredBy"; component: string };

export interface PlanView {
  changes: Change[];
  reasons: Record<string, Why>;
  downloadBytes: number;
  conflicts: Conflict[];
}

/** `engine::Mode`. */
export type Mode = "installWhileDownloading" | "downloadAllThenInstall";

/** `progress::Event`. */
export type ProgressEvent =
  | { kind: "started"; packages: number; bytes: number }
  | { kind: "downloading"; component: string; done: number; total: number | null }
  | { kind: "installing"; component: string; index: number; of: number }
  | { kind: "output"; component: string; line: string }
  | { kind: "installed"; component: string; version: string }
  | { kind: "removed"; component: string }
  | { kind: "finished"; ok: boolean; cancelled: boolean; error: string | null };



/** How far the Installer's own update has downloaded. */
export interface SelfUpdateProgress {
  done: number;
  total: number | null;
}

export const api = {
  snapshot: (refresh: boolean) => invokeCommand<Snapshot>(installer.installer.snapshot, { refresh }),
  selectionOf: (product: string) => invokeCommand<Selection>(installer.installer.selectionOf, { product }),
  plan: (product: string, selection: Selection) => invokeCommand<PlanView>(installer.installer.plan, { product, selection }),
  apply: (product: string, selection: Selection, mode: Mode, keepDownloads: boolean) => invokeCommand<void>(installer.installer.apply, { product, selection, mode, keepDownloads }),
  uninstall: (product: string) => invokeCommand<void>(installer.installer.uninstall, { product }),
  repair: (product: string) => invokeCommand<void>(installer.installer.repair, { product }),
  resume: () => invokeCommand<void>(installer.installer.resume),
  forgetPending: () => invokeCommand<void>(installer.installer.forgetPending),
  cancel: () => invokeCommand<void>(installer.installer.cancel),
  pause: (paused: boolean) => invokeCommand<void>(installer.installer.pause, { paused }),
  launch: (product: string) => invokeCommand<void>(installer.installer.launch, { product }),
  selfUpdate: () => invokeCommand<void>(installer.installer.selfUpdate),
  onSelfUpdate: (handler: (progress: SelfUpdateProgress) => void): Promise<UnlistenFn> =>
    listen<SelfUpdateProgress>(events.selfUpdate, (e) => handler(e.payload)),
  onProgress: (handler: (product: string, event: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<{ product: string; event: ProgressEvent }>(events.progress, (e) => handler(e.payload.product, e.payload.event)),
};
