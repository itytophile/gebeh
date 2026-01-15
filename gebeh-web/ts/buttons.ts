import { FromMainMessage } from "./common";

for (const element of document.querySelectorAll(".interactive")) {
  element.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
}

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
    element.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      port.postMessage({
        type: "input",
        event: "down",
        button,
      } satisfies FromMainMessage);
    });
    element.addEventListener("pointerup", () => {
      port.postMessage({
        type: "input",
        event: "up",
        button,
      } satisfies FromMainMessage);
    });
  }

  let dpadState = {
    left: false,
    right: false,
    up: false,
    down: false,
  };

  dpad.addEventListener("pointerdown", (event) => {
    event.preventDefault();

    const rect = dpad.getBoundingClientRect();
    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;

    const newDpadState = {
      left: x < 1 / 3,
      right: x > 2 / 3,
      up: y < 1 / 3,
      down: y > 2 / 3,
    };

    for (const [before, now, button] of [
      [dpadState.left, newDpadState.left, "left"],
      [dpadState.right, newDpadState.right, "right"],
      [dpadState.up, newDpadState.up, "up"],
      [dpadState.down, newDpadState.down, "down"],
    ] as const) {
      if (before !== now && now) {
        port.postMessage({
          type: "input",
          event: "down",
          button,
        } satisfies FromMainMessage);
      }
    }

    dpadState = newDpadState;
  });

  dpad.addEventListener("pointerup", () => {
    for (const [before, button] of [
      [dpadState.left, "left"],
      [dpadState.right, "right"],
      [dpadState.up, "up"],
      [dpadState.down, "down"],
    ] as const) {
      if (before) {
        port.postMessage({
          type: "input",
          event: "up",
          button,
        } satisfies FromMainMessage);
      }
    }
    dpadState = {
      left: false,
      right: false,
      up: false,
      down: false,
    };
  });
}
