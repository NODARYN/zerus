// @vitest-environment happy-dom
// zerus-uiy9p G15: the inspector names a task by its project with Open task,
// not by its UUID, and the base by a short SHA (the full values stay in the
// tooltips); a card's "#N" opens task N on the board. The board is the core's
// projection (client/core/src/teams/fixtures/board-pending.json); the card is
// the core's drawers fixture with a hand-written text naming two tasks.
import { afterEach, expect, it, vi } from "vitest";
import { createComponent } from "solid-js";
import { render } from "solid-js/web";
import type { TeamView } from "./types";
import type { DrawerRow } from "../../store/teamDrawers";
import fromCore from "../../../core/src/teams/fixtures/board-pending.json";
import drawers from "../../../core/src/teams/fixtures/drawers.json";

const nav = vi.hoisted(() => ({ openTask: vi.fn() }));
vi.mock("../../store/taskNavigation", () => ({ openTask: nav.openTask }));
import { TeamWorkspace } from "./TeamTab";
import { ProposalCards } from "./ProposalCards";
import { openTeamNode, setOpenTeamNode } from "./openNode";

let dispose: (() => void) | undefined;
afterEach(() => { dispose?.(); dispose = undefined; document.body.replaceChildren(); setOpenTeamNode(null); nav.openTask.mockReset(); });
const view = fromCore as unknown as TeamView;
const card = view.groups.flatMap((group) => group.cards).find((candidate) => candidate.task && candidate.assigned_base)!;

it("names the task by its project with Open task and the base by a short SHA", () => {
  const host = document.createElement("div"); document.body.append(host);
  dispose = render(() => createComponent(TeamWorkspace, { view, busy: false, failure: null, onAction: () => {}, onPage: () => {}, onAddTask: () => {} }), host);
  host.querySelector<HTMLButtonElement>(`.team-task-card[data-node="${card.node}"]`)!.click();
  const detail = host.querySelector<HTMLElement>("[data-team-detail]")!;
  expect(detail.textContent).not.toContain(card.task!.uuid);
  const row = detail.querySelector<HTMLElement>("[data-task-ref]")!;
  expect(row.textContent).toContain(card.task!.project);
  expect(row.getAttribute("title")).toContain(card.task!.uuid);
  row.querySelector<HTMLButtonElement>("[data-open-task]")!.click();
  expect(nav.openTask).toHaveBeenCalledExactlyOnceWith(card.task!.project, card.task!.uuid);
  const base = detail.querySelector<HTMLElement>("[data-assigned-base]")!;
  expect(base.textContent).toBe(card.assigned_base!.slice(0, 12));
  expect(base.getAttribute("title")).toBe(card.assigned_base);
});

it("handwritten_a_cards_task_numbers_open_those_tasks", () => {
  const row = (drawers.cards as { rows: DrawerRow[] }).rows.find((candidate) => candidate.card)!;
  const text = "The lead accepted #6. Every stage of #1 passed.";
  const closed = vi.fn();
  const host = document.createElement("div"); document.body.append(host);
  dispose = render(() => createComponent(ProposalCards, { rows: [{ ...row, card: { ...row.card!, text } }], busy: false, onAction: () => {}, team: view.bar.id, onOpenNode: closed }), host);
  const links = [...host.querySelectorAll<HTMLButtonElement>("[data-card-node]")];
  expect(links.map((link) => link.textContent)).toEqual(["#6", "#1"]);
  expect(host.querySelector(".team-drawer-text")?.textContent).toBe(text);
  links[0].click();
  expect(openTeamNode()).toEqual({ team: view.bar.id, node: 6 });
  expect(closed).toHaveBeenCalledOnce();
});
