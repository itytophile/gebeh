import * as wasm from "../pkg/index";

const proxy = wasm.init_window();

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
};
