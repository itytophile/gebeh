import { AUDIO_PROCESSOR_NAME, type FromMainMessage, type FromNodeMessage } from "./common.ts";
import { writeSave } from "./saves";
import workletURL from "./worklet.ts?worker&url";
import wasm from "../pkg/gebeh_web_bg.wasm?url";

async function initNode(): Promise<AudioWorkletNode> {
  const audioContext = new AudioContext();
  await audioContext.audioWorklet.addModule(workletURL);
  const node = new AudioWorkletNode(audioContext, AUDIO_PROCESSOR_NAME, {
    outputChannelCount: [2],
  });
  const { port } = node;
  return new Promise<AudioWorkletNode>((resolve) => {
    // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
    port.addEventListener("message", async ({ data }: MessageEvent<FromNodeMessage>) => {
      switch (data.type) {
        case "ready": {
          // ready
          resolve(node);
          break;
        }
        case "wasm": {
          console.log("Sending wasm");
          // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
          void fetch(wasm)
            .then((response) => response.bytes())
            .then((bytes) => {
              port.postMessage({ type: "wasm", bytes } satisfies FromMainMessage, [bytes.buffer]);
            });
          break;
        }
        case "save": {
          await writeSave(data.title, data.buffer);
        }
      }
    });
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState == "visible") {
        port.postMessage({
          type: "enableMessages",
        } satisfies FromMainMessage);
      } else {
        port.postMessage({
          type: "disableMessages",
        } satisfies FromMainMessage);
      }
    });
    port.start();
    node.connect(audioContext.destination);
  });
}
export default initNode;
