import { init_window, Sampler } from "../pkg/index";

let gebehNode: AudioWorkletNode | undefined;

const proxy = init_window((sampler: unknown) => {
  if (!(sampler instanceof Sampler)) {
    throw new Error("Not Sampler");
  }
  gebehNode?.port.postMessage(false);
});

const romInput = document.getElementById("rom-input");

if (!(romInput instanceof HTMLInputElement)) {
  throw new Error("rom-input is not an input");
}

romInput.onchange = async () => {
  const file = romInput.files?.item(0);
  if (!file) {
    return;
  }
  proxy.send_file(await file.bytes());
  // const audioContext = new AudioContext();
  // await audioContext.audioWorklet.addModule("./gebeh-audio-processor.js");
  // gebehNode = new AudioWorkletNode(audioContext, "gebeh-processor");
  // gebehNode.connect(audioContext.destination);
};
