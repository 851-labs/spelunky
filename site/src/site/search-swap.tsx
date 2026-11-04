// The landing page's header search box becoming the wiki's (src/site/frame.tsx's SiteSearch, 851-4004): this chunk,
// with the wiki's box (src/wiki/search-box.tsx) and Base UI, loads the first time the plain form is focused or
// pressed, and puts the wiki's box in its place with what's typed, the selection and the focus, so the swap doesn't
// show. All of it here, rather than in the landing page's scripts up front. When:
//
//   - not while an input method's composition, or a press on the form (a click, a tap, a drag to select), is under
//     way, which it would cut off: after it, once the press's click and focus have landed;
//   - with the focus in the form on a device with only touch, at the reader's next edit, in its own event: a phone
//     raises its keyboard for a box focused in a reader's event, and might not for one focused by a script outside
//     one (with a mouse or a trackpad there's no such keyboard, and what's typed already gets its list at once);
//   - else at once.
//
// It returns a cancel, for the form's unmounting before then.
import { flushSync } from "react-dom";
import type { ReactNode } from "react";
import type { BoxState } from "../wiki/search-box";
import { SearchBox } from "../wiki/search-box";

// After a press is let go: by then its click and focus have landed, a touch's too.
const AFTER_PRESS = 400;
// A device with only touch, as landing.css's touch note has it: no fine pointer anywhere.
const touchOnly = () => matchMedia("(pointer: coarse)").matches && !matchMedia("(any-pointer: fine)").matches;

export function swap(form: HTMLFormElement | null, set: (box: ReactNode) => void): () => void {
  const input = form?.querySelector("input");
  const button = form?.querySelector("button");
  if (!form || !input) return () => {};
  let done = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const go = (event?: Event) => {
    if (done || !form.isConnected) return;
    const edit = event?.type === "input" || event?.type === "compositionend";
    if (form.hasAttribute("data-composing") && event?.type !== "compositionend") return;
    if (form.matches(":active")) return;
    if (form.contains(document.activeElement) && !edit && touchOnly()) return;
    done = true;
    cancel();
    const active = document.activeElement;
    const from: BoxState = {
      value: input.value,
      focus: active === input ? "input" : active === button ? "button" : null,
      start: input.selectionStart,
      end: input.selectionEnd,
      direction: input.selectionDirection,
    };
    // At once, in the event: no key lands in the old box in between, and the new one is focused in it.
    flushSync(() => set(<SearchBox from={from} />));
  };
  const later = (delay: number) => () => {
    clearTimeout(timer);
    timer = setTimeout(go, delay);
  };
  const listeners: [EventTarget, string, EventListener][] = [
    [form, "input", go],
    [form, "compositionend", go],
    // A key let go (a held Space's :active), and the focus leaving the form, once it has.
    [form, "keyup", go],
    [form, "focusout", later(0)],
    [window, "pointerup", later(AFTER_PRESS)],
    [window, "pointercancel", later(AFTER_PRESS)],
  ];
  const cancel = () => {
    clearTimeout(timer);
    for (const [target, type, listener] of listeners) target.removeEventListener(type, listener, true);
  };
  for (const [target, type, listener] of listeners) target.addEventListener(type, listener, true);
  go();
  return cancel;
}
