import { AUDIO_PROCESSOR_NAME, FromMainMsg, FromNodeMsg } from "./common.js";

const romInput = document.getElementById("rom-input");

if (!(romInput instanceof HTMLInputElement)) {
  throw new Error("rom-input is not an input");
}

romInput.onchange = async () => {
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
    } satisfies FromMainMsg);
  } else {
    notReadyRom = bytes;
  }
};

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
  port.onmessage = ({ data }: MessageEvent<FromNodeMsg>) => {
    switch (data.type) {
      case "ready": {
        // ready
        isNodeReady = true;
        if (notReadyRom) {
          console.log("sending");
          port.postMessage(
            { type: "rom", bytes: notReadyRom } satisfies FromMainMsg,
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
            port.postMessage({ type: "wasm", bytes } satisfies FromMainMsg, [
              bytes,
            ]);
          });
      }
    }
  };
  console.log("allo les boys");
  node.connect(audioContext.destination);
  console.log("je sais pas trop");
  return node;
}
