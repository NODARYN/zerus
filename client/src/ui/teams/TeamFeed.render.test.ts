// @vitest-environment happy-dom
// The Feed reads as sentences (zerus-rzvct G6): each journal event names who
// acted (you, the team service or a member), what happened, and the task and
// member it is about, on separate elements; the exact journal text stays as
// the row's title. Rows are the core's projection of the product-written
// journal (crates/team-plan/fixtures/cli-journal.json).
import { afterEach, expect, it } from "vitest";
import { createComponent } from "solid-js";
import { render } from "solid-js/web";
import { TeamFeed } from "./TeamDrawers";
import pages from "../../../core/src/teams/fixtures/drawers.json";
import type { DrawerPage, DrawerRow } from "../../store/teamDrawers";

let dispose: (() => void) | undefined;
afterEach(() => { dispose?.(); dispose = undefined; document.body.replaceChildren(); });
const feed = pages.feed as DrawerPage;
function show(rows: DrawerRow[]) {
  const host = document.createElement("div"); document.body.append(host);
  dispose = render(() => createComponent(TeamFeed, { rows }), host);
  return host;
}
const read = (host: HTMLElement, seq: number) => {
  const row = host.querySelector(`[data-feed-seq="${seq}"]`)!;
  const text = (selector: string) => row.querySelector(selector)?.textContent ?? null;
  return { actor: text("[data-feed-actor]"), sentence: text("[data-feed-sentence]"), task: text("[data-feed-task]"), member: text("[data-feed-member]"), title: row.getAttribute("title") };
};

it("says who did what to which task and member, in separate elements", () => {
  const host = show(feed.rows);
  expect(read(host, 1)).toEqual({ actor: "You", sentence: "Team created", task: null, member: null, title: "create at revision 1" });
  expect(read(host, 2)).toEqual({ actor: "Team service", sentence: "Member session linked", task: null, member: "lead", title: "set_member_session at revision 2" });
  expect(read(host, 5)).toMatchObject({ actor: "Team service", sentence: "Task state observed", task: "#1", member: null });
  expect(read(host, 8)).toMatchObject({ actor: "Team service", sentence: "Hand-out prepared", task: "#2", member: "m1" });
  expect(read(host, 13)).toMatchObject({ sentence: "Report recorded", task: "#2", member: "m1" });
  expect(read(host, 15)).toMatchObject({ actor: "m1", sentence: "Question asked", task: "#2" });
  expect(read(host, 16)).toMatchObject({ actor: "You", sentence: "Question answered" });
  const header = host.querySelector('[data-feed-seq="1"] header');
  expect(header?.querySelector("time")).not.toBeNull();
  expect(header?.querySelector("[data-feed-actor]")).not.toBeNull();
});

it("never shows an op code: every product op has a sentence and an unknown one reads as words", () => {
  const host = show(feed.rows);
  for (const row of feed.rows) expect(read(host, row.seq).sentence, row.kind).not.toContain("_");
  dispose?.(); document.body.replaceChildren();
  const later = show([{ ...feed.rows[0], seq: 99, kind: "some_future_op", text: "some_future_op at revision 99" }]);
  expect(read(later, 99).sentence).toBe("some future op");
});
