import { useRef } from "react";
import type { FromMainMessage } from "./common";
import dpad from "../assets/dpad.svg";

function Dpad({ port }: { port: MessagePort }) {
  const isPointerDown = useRef(false);
  const dpadState = useRef({
    left: false,
    right: false,
    up: false,
    down: false,
  });

  function applyDpadState(newDpadState: {
    left: boolean;
    right: boolean;
    up: boolean;
    down: boolean;
  }) {
    for (const [before, now, button] of [
      [dpadState.current.left, newDpadState.left, "left"],
      [dpadState.current.right, newDpadState.right, "right"],
      [dpadState.current.up, newDpadState.up, "up"],
      [dpadState.current.down, newDpadState.down, "down"],
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

    dpadState.current = newDpadState;
  }

  function cancelPointer(event: React.PointerEvent<HTMLImageElement>) {
    isPointerDown.current = false;
    event.currentTarget.releasePointerCapture(event.pointerId);
    applyDpadState({ down: false, left: false, right: false, up: false });
  }

  function handlePress(event: React.PointerEvent<HTMLImageElement>) {
    if (!isPointerDown.current) {
      return;
    }

    const rect = event.currentTarget.getBoundingClientRect();

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

  return (
    <img
      className="interactive"
      src={dpad}
      onPointerDown={(event) => {
        isPointerDown.current = true;
        event.preventDefault();
        // to be able to move the finger while pressing and change buttons
        event.currentTarget.setPointerCapture(event.pointerId);

        handlePress(event);
      }}
    />
  );
}

export default Dpad;
