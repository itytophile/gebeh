import {
  AUDIO_PROCESSOR_NAME,
  FromMainMessage,
  FromNodeMessage,
} from "./common.js";

const romInput = document.querySelector("#rom-input");

if (!(romInput instanceof HTMLInputElement)) {
  throw new TypeError("rom-input is not an input");
}

romInput.addEventListener("change", async () => {
  const file = romInput.files?.item(0);

  if (!file) {
    return;
  }

  console.log("issou");

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

async function getAudioWorkletNode(): Promise<AudioWorkletNode> {
  if (node) {
    return node;
  }
  const audioContext = new AudioContext();
  await audioContext.audioWorklet.addModule("dist/worklet.js");
  node = new AudioWorkletNode(audioContext, AUDIO_PROCESSOR_NAME, {
    outputChannelCount: [2],
  });
  const { port } = node;
  // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
  port.addEventListener(
    "message",
    ({ data }: MessageEvent<FromNodeMessage>) => {
      switch (data.type) {
        case "ready": {
          // ready
          isNodeReady = true;
          if (notReadyRom) {
            console.log("sending");
            port.postMessage(
              { type: "rom", bytes: notReadyRom } satisfies FromMainMessage,
              [notReadyRom],
            );
            console.log("sent");
          }
          break;
        }
        case "wasm": {
          // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
          void fetch("pkg/gebeh_web_bg.wasm")
            .then((response) => response.arrayBuffer())
            .then((bytes) => {
              port.postMessage(
                { type: "wasm", bytes } satisfies FromMainMessage,
                [bytes],
              );
            });
        }
      }
    },
  );
  console.log("allo les boys");
  node.connect(audioContext.destination);
  console.log("je sais pas trop");
  return node;
}

const canvas = document.querySelector("#canvas");

if (!(canvas instanceof HTMLCanvasElement)) {
  throw new TypeError("Not Canvas");
}

const context = canvas.getContext("2d");

if (!context) {
  throw new Error("Canvas context is null");
}

const imageData = context.createImageData(100, 100);

// Iterate through every pixel
for (let index = 0; index < imageData.data.length; index += 4) {
  // Modify pixel data
  imageData.data[index + 0] = 190; // R value
  imageData.data[index + 1] = 0; // G value
  imageData.data[index + 2] = 210; // B value
  imageData.data[index + 3] = 255; // A value
}

// Draw image data to the canvas
context.putImageData(imageData, 20, 20);
