import { useRef } from "react";
import type { FromMainMessage } from "./common";
import dpad from "../assets/dpad.svg";
import style from "../style.module.css";

function Dpad({ port }: { port: MessagePort }) {
  const dpadState = useRef({
    left: false,
    right: false,
    up: false,
    down: false,
  });

  const applyDpadState = (nextState: typeof dpadState.current) => {
    for (const key of ["left", "right", "up", "down"] as const) {
      const before = dpadState.current[key];
      const now = nextState[key];

      if (before !== now) {
        port.postMessage({
          type: "input",
          event: now ? "down" : "up",
          button: key,
        } satisfies FromMainMessage);
      }
    }

    dpadState.current = nextState;
  };

  const updateButtonsFromEvent = (event: React.PointerEvent<HTMLImageElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();

    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;

    const newState = {
      left: x < 1 / 3,
      right: x > 2 / 3,
      up: y < 1 / 3,
      down: y > 2 / 3,
    };

    applyDpadState(newState);
  };

  const handlePointerUpOrLeave = () => {
    applyDpadState({ left: false, right: false, up: false, down: false });
  };

  return (
    <img
      className={style.interactive}
      src={dpad}
      onPointerDown={(event: React.PointerEvent<HTMLImageElement>) => {
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        updateButtonsFromEvent(event);
      }}
      onPointerMove={(event: React.PointerEvent<HTMLImageElement>) => {
        if (event.buttons > 0) {
          updateButtonsFromEvent(event);
        }
      }}
      onPointerUp={handlePointerUpOrLeave}
      onPointerCancel={handlePointerUpOrLeave}
    />
  );
}

export default Dpad;
