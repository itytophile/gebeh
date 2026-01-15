import { FromMainMessage, GebehButton } from "./common";

export function addInputs(canvas: HTMLCanvasElement, port: MessagePort) {
  canvas.addEventListener("keydown", (event) => {
    if (event.repeat) {
      return;
    }
    const button = input_mapping(event.key);
    if (!button) {
      return;
    }
    port.postMessage({
      type: "input",
      event: "down",
      button,
    } satisfies FromMainMessage);
  });
  canvas.addEventListener("keyup", (event) => {
    const button = input_mapping(event.key);
    if (!button) {
      return;
    }
    port.postMessage({
      type: "input",
      event: "up",
      button,
    } satisfies FromMainMessage);
  });
}

function input_mapping(key: string): GebehButton | undefined {
  const lowerCase = key.toLocaleLowerCase();
  switch (lowerCase) {
    case "a":
    case "b": {
      return lowerCase;
    }
    case "enter": {
      return "start";
    }
    case "backspace": {
      return "select";
    }
    case "arrowleft": {
      return "left";
    }
    case "arrowright": {
      return "right";
    }
    case "arrowup": {
      return "up";
    }
    case "arrowdown": {
      return "down";
    }
  }
}
