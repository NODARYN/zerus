// Adapted from WhiteWh's history.ts helpers. Our journal has its own text and
// actor, not the fork engine's `detail` payload; never infer missing events.
import { tr } from "../../i18n";
import type { DrawerRow } from "../../store/teamDrawers";

export function feedClock(atMs: number): string {
  if (!atMs) return "";
  const date = new Date(atMs);
  return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

export function inboxKind(kind: string): string {
  switch (kind.split(":")[0]) {
    case "report": return tr("Report");
    case "provider_stop": return tr("Provider stop");
    case "question": return tr("Question");
    case "permission": return tr("Permission");
    case "planning_goal": return tr("Planning goal");
    default: return kind;
  }
}

export function inboxText(row: DrawerRow): string {
  if (!row.kind.startsWith("unstaffed:") || row.text_truncated) return row.text;
  // Retain the stored English sentence for older CLI readers. Translate
  // only the service's exact notice, with IDs kept as data.
  const notice = /^(Task|Pipeline stage) ([1-9][0-9]*) is ready\. The service cannot read any eligible member \(([a-zA-Z0-9_-]+(?:, [a-zA-Z0-9_-]+)*)\)\. It will check again\.$/.exec(row.text);
  if (!notice || notice[2] !== String(row.node)) return row.text;
  const [, subject, node, members] = notice;
  return subject === "Pipeline stage"
    ? tr("Pipeline stage %1 is ready. The service cannot read any eligible member (%2). It will check again.", node, members)
    : tr("Task %1 is ready. The service cannot read any eligible member (%2). It will check again.", node, members);
}

export const reportRows = (rows: readonly DrawerRow[]) => rows.filter((row) => row.kind === "report");
export const historyLine = (row: DrawerRow) => row.text || row.kind;
