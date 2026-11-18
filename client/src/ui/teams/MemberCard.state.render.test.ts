// @vitest-environment happy-dom
// zerus-uiy9p G9: a member's state says what it is and why an operation is
// closed. A member the person stopped is stored paused with person_stopped;
// its card says Stopped, not Paused. A disabled Swap or Stop shows its reason
// under the item, not only in a hover title. The members are hand-written
// variants of the card tests' member (no product fixture holds a person-stopped
// member); the reasons are the product's own swapBlocked/memberActions texts.
import { afterEach, expect, it, vi } from "vitest";
import { createComponent } from "solid-js";
import { render } from "solid-js/web";
import { MenuLayer } from "../components/Overlays";
import { setMenu } from "../../store/ui";
import { MemberCard } from "./MemberCard";
import { swapBlocked } from "./member-edit";
import type { MemberView } from "./types";

let dispose: (() => void) | undefined;
afterEach(() => { dispose?.(); dispose = undefined; document.body.replaceChildren(); setMenu(undefined); });

const member = (over: Partial<MemberView> = {}): MemberView => ({
  id: "m1", role: "developer", agent: "claude", account: "account-example", state: "active", lead: false, open_work: 2,
  session: { machine: "00000000-0000-4000-8000-0000000000aa", session_id: "launch_id:00000000-0000-4000-8000-000000000031" },
  ...over,
});
function setup(value: MemberView) {
  const host = document.createElement("div");
  document.body.append(host);
  dispose = render(() => [createComponent(MemberCard, { teamStatus: "running", member: value, busy: false, onAction: vi.fn() }), createComponent(MenuLayer, {})], host);
  return host;
}
const openMenu = (host: HTMLElement) => (host.querySelector('[aria-label="Member actions"]') as HTMLButtonElement).click();
const item = (host: HTMLElement, label: string) => [...host.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((button) => button.textContent === label)!;
const note = (host: HTMLElement, button: HTMLButtonElement) => {
  const id = button.getAttribute("aria-describedby");
  return id ? host.ownerDocument.getElementById(id)?.textContent : undefined;
};

it("handwritten_a_member_the_person_stopped_says_stopped_not_paused", () => {
  const stopped = setup(member({ state: "paused", person_stopped: true, open_work: 0 }));
  const badge = stopped.querySelector(".team-member-state")!;
  expect(badge.textContent).toBe("stopped");
  expect(badge.getAttribute("data-tone")).toBe("danger");
  expect(stopped.querySelector("[data-member]")?.getAttribute("data-state")).toBe("stopped");
  dispose?.(); document.body.replaceChildren();
  const paused = setup(member({ state: "paused", person_stopped: false }));
  expect(paused.querySelector(".team-member-state")?.textContent).toBe("paused");
  expect(paused.querySelector(".team-member-state")?.getAttribute("data-tone")).toBe("warn");
});

it("handwritten_a_closed_swap_shows_why_under_the_item", () => {
  const value = member();
  const host = setup(value);
  openMenu(host);
  const swap = item(host, "Swap…");
  expect(swap.getAttribute("aria-disabled")).toBe("true");
  expect(note(host, swap)).toBe(swapBlocked(value));
  expect(note(host, swap)).toBe("Stop the member first; open work: 2");
  // An open item carries no note.
  expect(item(host, "Pause").getAttribute("aria-describedby")).toBeNull();
});

it("handwritten_a_closed_stop_of_a_member_the_person_stopped_shows_why", () => {
  const host = setup(member({ state: "paused", person_stopped: true, open_work: 0 }));
  openMenu(host);
  const stop = item(host, "Stop");
  expect(stop.getAttribute("aria-disabled")).toBe("true");
  expect(note(host, stop)).toBe("You stopped this member. Resume it to let it work again.");
  // The stopped member with no open work may be swapped: no note, not closed.
  expect(item(host, "Swap…").getAttribute("aria-disabled")).not.toBe("true");
  expect(item(host, "Swap…").getAttribute("aria-describedby")).toBeNull();
});

it("the member line wraps its state and menu under the name instead of squeezing it", async () => {
  const { readFileSync } = await import("node:fs");
  // Vitest serves CSS imports empty: the stylesheets are read as they ship.
  for (const file of ["teams.css", "team-member-link.css"]) {
    const style = document.createElement("style");
    // happy-dom skips rules inside @layer: the one layer wrapper is unwrapped, the rules stay as shipped.
    const css = readFileSync(`${process.cwd()}/src/ui/styles/${file}`, "utf8");
    const layer = css.match(/@layer\s+[\w-]+\s*\{/);
    style.textContent = layer ? css.slice(0, layer.index) + css.slice(layer.index! + layer[0].length, css.lastIndexOf("}")) : css;
    document.head.append(style);
  }
  try {
    const host = setup(member({ state: "paused", person_stopped: false }));
    const line = host.querySelector<HTMLElement>(".team-member-line")!;
    expect(getComputedStyle(line).flexWrap).toBe("wrap");
    expect(getComputedStyle(host.querySelector<HTMLElement>(".team-member-state")!).flexShrink).toBe("0");
  } finally { document.head.replaceChildren(); }
});
