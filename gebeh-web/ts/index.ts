// try to not import wasm functions here (let's have some fun)

import { addButtons } from "./buttons.js";
import {
  AUDIO_PROCESSOR_NAME,
  FromMainMessage,
  FromNodeMessage,
  GB_HEIGHT,
  GB_WIDTH,
} from "./common.js";
import { addInputs } from "./keyboard.js";
import { addNetwork } from "./network.js";
import { getSave, writeSave } from "./saves.js";

const toolbar = document.getElementById("toolbar");

if (!(toolbar instanceof HTMLDivElement)) {
  throw new TypeError("toolbar is not a div");
}

const romInput = document.getElementById("rom-input");

if (!(romInput instanceof HTMLInputElement)) {
  throw new TypeError("rom-input is not an input");
}

romInput.addEventListener("change", async () => {
  const file = romInput.files?.item(0);

  if (!file) {
    return;
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  const node = await getAudioWorkletNode();

  // toolbar.classList.add("hidden");

  if (isNodeReady) {
    const save = await getSave(getTitleFromRom(new Uint8Array(bytes)));
    node.port.postMessage(
      {
        type: "rom",
        bytes,
        save,
      } satisfies FromMainMessage,
      save ? [bytes.buffer, save.buffer] : [bytes.buffer],
    );
  } else {
    notReadyRom = bytes;
  }
});

let node: AudioWorkletNode | undefined;
let isNodeReady = false;
let notReadyRom: Uint8Array | undefined;

const canvas = document.getElementById("canvas");

if (!(canvas instanceof HTMLCanvasElement)) {
  throw new TypeError("Not Canvas");
}

const context = canvas.getContext("2d");

if (!context) {
  throw new Error("Canvas context is null");
}

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
  addInputs(canvas, port);
  console.log("wouf");
  addNetwork(port);
  addButtons(port);
  // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
  port.addEventListener(
    "message",
    async ({ data }: MessageEvent<FromNodeMessage>) => {
      switch (data.type) {
        case "ready": {
          // ready
          isNodeReady = true;
          if (notReadyRom) {
            const save = await getSave(
              getTitleFromRom(new Uint8Array(notReadyRom)),
            );
            port.postMessage(
              {
                type: "rom",
                bytes: notReadyRom,
                save,
              } satisfies FromMainMessage,
              save ? [notReadyRom.buffer, save.buffer] : [notReadyRom.buffer],
            );
          }
          break;
        }
        case "wasm": {
          console.log("Sending wasm");
          // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
          void fetch("pkg/gebeh_web_bg.wasm")
            .then((response) => response.bytes())
            .then((bytes) => {
              port.postMessage(
                { type: "wasm", bytes } satisfies FromMainMessage,
                [bytes.buffer],
              );
            });
          break;
        }
        case "frame": {
          for (const [index, byte] of new Uint8Array(data.buffer).entries()) {
            for (let index_2bits = 0; index_2bits < 4; ++index_2bits) {
              const gray = (((byte >> (6 - 2 * index_2bits)) & 0b11) * 255) / 3;
              const index_color = (index * 4 + index_2bits) * 4;
              const data = imageData.data;
              data[index_color] = gray;
              data[index_color + 1] = gray;
              data[index_color + 2] = gray;
              data[index_color + 3] = 255;
            }
          }
          context.putImageData(imageData, 0, 0);
          break;
        }
        case "save": {
          await writeSave(data.title, data.buffer);
        }
      }
    },
  );
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState == "visible") {
      port.postMessage({ type: "enableMessages" } satisfies FromMainMessage);
    } else {
      port.postMessage({ type: "disableMessages" } satisfies FromMainMessage);
    }
  });
  port.start();
  node.connect(audioContext.destination);
  return node;
};

function getTitleFromRom(rom: Uint8Array): string {
  const title = rom.slice(0x134, 0x143);

  let endZeroPos = title.indexOf(0);
  if (endZeroPos === -1) {
    endZeroPos = title.length;
  }

  const decoder = new TextDecoder("utf-8", { fatal: true });
  return decoder.decode(title.slice(0, endZeroPos));
}
