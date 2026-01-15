import { FromMainMessage } from "./common";

function getButton(id: string) {
  const button = document.getElementById(id);
  if (!button) {
    throw new TypeError("Can't find element with id = " + id);
  }
  return button;
}

export function addButtons(port: MessagePort) {
  const dpad = getButton("dpad");
  const buttonA = getButton("buttonA");
  const buttonB = getButton("buttonB");
  const buttonStart = getButton("buttonStart");
  const buttonSelect = getButton("buttonSelect");

  for (const [element, button] of [
    [buttonA, "a"],
    [buttonB, "b"],
    [buttonStart, "start"],
    [buttonSelect, "select"],
  ] as const) {
    element.addEventListener("touchstart", () => {
      port.postMessage({
        type: "input",
        event: "down",
        button,
      } satisfies FromMainMessage);
    });
    element.addEventListener("touchend", () => {
      port.postMessage({
        type: "input",
        event: "up",
        button,
      } satisfies FromMainMessage);
    });
  }

  dpad.addEventListener("touchstart", (event) => {
    touchDpad(dpad.getBoundingClientRect(), event, port, true);
  });

  dpad.addEventListener("touchend", (event) => {
    touchDpad(dpad.getBoundingClientRect(), event, port, false);
  });
}

let dpadState = {
  left: false,
  right: false,
  up: false,
  down: false,
};

function touchDpad(
  rect: DOMRect,
  event: TouchEvent,
  port: MessagePort,
  isStart: boolean,
) {
  const newDpadState = {
    left: false,
    right: false,
    up: false,
    down: false,
  };

  for (const touch of event.touches) {
    const x = (touch.clientX - rect.left) / rect.width;
    const y = (touch.clientY - rect.top) / rect.height;

    newDpadState.left ||= x < 1 / 3;
    newDpadState.right ||= x > 2 / 3;
    newDpadState.up ||= y < 1 / 3;
    newDpadState.down ||= y > 2 / 3;
  }

  for (const [before, now, button] of [
    [dpadState.left, newDpadState.left, "left"],
    [dpadState.right, newDpadState.right, "right"],
    [dpadState.up, newDpadState.up, "up"],
    [dpadState.down, newDpadState.down, "down"],
  ] as const) {
    if (before !== now && isStart === now) {
      port.postMessage({
        type: "input",
        event: isStart ? "down" : "up",
        button,
      } satisfies FromMainMessage);
    }
  }

  dpadState = newDpadState;
}
