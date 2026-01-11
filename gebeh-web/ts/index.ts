import init, { init_audio, init_window } from "../pkg/gebeh_web.js";

await init();
const proxy = init_window();

const romInput = document.getElementById("rom-input");

if (!(romInput instanceof HTMLInputElement)) {
  throw new Error("rom-input is not an input");
}

romInput.onchange = async () => {
  const file = romInput.files?.item(0);
  if (!file) {
    return;
  }
  init_audio();
  proxy.send_file(await file.bytes());
};
