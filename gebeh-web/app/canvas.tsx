import { useEffect, useRef } from "react";
import {
  GB_HEIGHT,
  GB_WIDTH,
  type FromMainMessage,
  type FromNodeMessage,
  type GebehButton,
} from "./common";

function addInputs(canvas: HTMLCanvasElement, port: MessagePort) {
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

const initCanvas = (canvas: HTMLCanvasElement, port: MessagePort) => {
  const context = canvas.getContext("2d");

  if (!context) {
    throw new Error("Canvas context is null");
  }

  const imageData = context.createImageData(GB_WIDTH, GB_HEIGHT);
  imageData.data.fill(0xaa);
  context.putImageData(imageData, 0, 0);

  addInputs(canvas, port);

  port.addEventListener("message", ({ data }: MessageEvent<FromNodeMessage>) => {
    switch (data.type) {
      case "frame": {
        const imageData = context.getImageData(0, 0, GB_WIDTH, GB_HEIGHT);
        const d = imageData.data;
        for (const [index, byte] of data.buffer.entries()) {
          const index_color = index * 4;
          const r = byte & 0x1f;
          const g = (byte >> 5) & 0x1f;
          const b = (byte >> 10) & 0x1f;
          d[index_color] = (r << 3) | (r >> 2);
          d[index_color + 1] = (g << 3) | (g >> 2);
          d[index_color + 2] = (b << 3) | (b >> 2);
          d[index_color + 3] = 255;
        }
        context.putImageData(imageData, 0, 0);
        break;
      }
    }
  });
};

function Canvas({ port }: { port: MessagePort }) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvas.current) {
      throw new TypeError("No canvas");
    }
    initCanvas(canvas.current, port);
  }, [port]);

  return <canvas ref={canvas} tabIndex={1} width="160" height="144" />;
}

export default Canvas;
