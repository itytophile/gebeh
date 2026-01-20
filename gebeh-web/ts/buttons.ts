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

  let isPointerDown = false;

  dpad.addEventListener("pointerdown", (event) => {
    isPointerDown = true;
    event.preventDefault();
    // to be able to move the finger while pressing and change buttons
    dpad.setPointerCapture(event.pointerId);

    handlePress(event);
  });

  dpad.addEventListener("pointermove", handlePress);
  dpad.addEventListener("pointerup", cancelPointer);
  dpad.addEventListener("pointercancel", cancelPointer);

  function cancelPointer(event: PointerEvent) {
    isPointerDown = false;
    dpad.releasePointerCapture(event.pointerId);
    applyDpadState({ down: false, left: false, right: false, up: false });
  }

  function handlePress(event: PointerEvent) {
    if (!isPointerDown) {
      return;
    }

    const rect = dpad.getBoundingClientRect();

    const inside =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;

    if (!inside) {
      cancelPointer(event);
      return;
    }

    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;

    applyDpadState({
      left: x < 1 / 3,
      right: x > 2 / 3,
      up: y < 1 / 3,
      down: y > 2 / 3,
    });
  }

  function applyDpadState(newDpadState: {
    left: boolean;
    right: boolean;
    up: boolean;
    down: boolean;
  }) {
    for (const [before, now, button] of [
      [dpadState.left, newDpadState.left, "left"],
      [dpadState.right, newDpadState.right, "right"],
      [dpadState.up, newDpadState.up, "up"],
      [dpadState.down, newDpadState.down, "down"],
    ] as const) {
      if (before === now) {
        continue;
      }
      port.postMessage({
        type: "input",
        event: now ? "down" : "up",
        button,
      } satisfies FromMainMessage);
    }

    dpadState = newDpadState;
  }
}
