// try to not import wasm functions here (let's have some fun)

import {
  AUDIO_PROCESSOR_NAME,
  FromMainMessage,
  FromNodeMessage,
} from "./common.js";
import { add_inputs } from "./inputs.js";

const romInput = document.querySelector("#rom-input");

if (!(romInput instanceof HTMLInputElement)) {
  throw new TypeError("rom-input is not an input");
}

romInput.addEventListener("change", async () => {
  const file = romInput.files?.item(0);

  if (!file) {
    return;
  }

  const bytes = await file.arrayBuffer();
  const node = await getAudioWorkletNode();
  if (isNodeReady) {
    node.port.postMessage({
      type: "rom",
      bytes,
    } satisfies FromMainMessage);
  } else {
    notReadyRom = bytes;
  }
});

let node: AudioWorkletNode | undefined;
let isNodeReady = false;
let notReadyRom: ArrayBuffer | undefined;

const canvas = document.querySelector("#canvas");

if (!(canvas instanceof HTMLCanvasElement)) {
  throw new TypeError("Not Canvas");
}

const context = canvas.getContext("2d");

if (!context) {
  throw new Error("Canvas context is null");
}

const GB_WIDTH = 160;
const GB_HEIGHT = 144;
const imageData = context.createImageData(GB_WIDTH, GB_HEIGHT);
imageData.data.fill(0xaa);
context.putImageData(imageData, 0, 0);

const getAudioWorkletNode = async (): Promise<AudioWorkletNode> => {
  if (node) {
    return node;
  }
  const audioContext = new AudioContext();
  await audioContext.audioWorklet.addModule("dist/worklet.js");
  node = new AudioWorkletNode(audioContext, AUDIO_PROCESSOR_NAME, {
    outputChannelCount: [2],
  });
  const { port } = node;
  add_inputs(canvas, port);
  // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
  port.addEventListener(
    "message",
    ({ data }: MessageEvent<FromNodeMessage>) => {
      switch (data.type) {
        case "ready": {
          // ready
          isNodeReady = true;
          if (notReadyRom) {
            port.postMessage(
              { type: "rom", bytes: notReadyRom } satisfies FromMainMessage,
              [notReadyRom],
            );
          }
          break;
        }
        case "wasm": {
          console.log("Sending wasm");
          // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
          void fetch("pkg/gebeh_web_bg.wasm")
            .then((response) => response.arrayBuffer())
            .then((bytes) => {
              port.postMessage(
                { type: "wasm", bytes } satisfies FromMainMessage,
                [bytes],
              );
            });
          break;
        }
        case "frame": {
          for (const [index, value] of data.bytes.entries()) {
            const offset = index * 4;
            const color =
              value === 0 ? 0xff : value === 1 ? 0xaa : value === 2 ? 0x55 : 0;
            imageData.data[offset] = color; // R value
            imageData.data[offset + 1] = color; // G value
            imageData.data[offset + 2] = color; // B value
            imageData.data[offset + 3] = 255; // A value
          }
          context.putImageData(imageData, 0, 0);
        }
      }
    },
  );
  port.start();
  node.connect(audioContext.destination);
  return node;
};
