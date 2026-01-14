import "../polyfill/TextEncoder.js";
import { initSync, WebEmulator } from "../pkg/gebeh_web.js";
import {
  AUDIO_PROCESSOR_NAME,
  FromMainMessage,
  FromNodeMessage,
} from "./common.js";

// https://github.com/microsoft/TypeScript-DOM-lib-generator/blob/0f96fae53f776b5d914c404ce611b4d16a921cb6/baselines/audioworklet.generated.d.ts
// I copied the declarations because doing something clean with multiple tsconfig files or whatever is too difficult
declare global {
  var sampleRate: number;

  interface AudioWorkletProcessor {
    readonly port: MessagePort;
  }
  var AudioWorkletProcessor: {
    prototype: AudioWorkletProcessor;
    new (): AudioWorkletProcessor;
  };
  interface AudioWorkletProcessorImpl extends AudioWorkletProcessor {
    process(
      inputs: Float32Array[][],
      outputs: Float32Array[][],
      parameters: Record<string, Float32Array>,
    ): boolean;
  }
  type AudioWorkletProcessorConstructor = new (
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    options: any,
  ) => AudioWorkletProcessorImpl;
  function registerProcessor(
    name: string,
    processorCtor: AudioWorkletProcessorConstructor,
  ): void;
}

class WasmProcessor
  extends AudioWorkletProcessor
  implements AudioWorkletProcessorImpl
{
  emulator?: WebEmulator;

  constructor() {
    super();
    this.port.addEventListener(
      "message",
      ({ data }: MessageEvent<FromMainMessage>) => {
        switch (data.type) {
          case "rom": {
            this.emulator?.load_rom(new Uint8Array(data.bytes));
            break;
          }
          case "wasm": {
            console.log("Initializing wasm");
            initSync({ module: data.bytes });
            this.emulator = new WebEmulator();
            this.port.postMessage({ type: "ready" } satisfies FromNodeMessage);
            console.log("Ready!");
            break;
          }
          case "input": {
            // inputs are inverted, so if up then we set to true to disable
            const is_up = data.event === "up";
            switch (data.button) {
              case "a": {
                this.emulator?.set_a(is_up);
                break;
              }
              case "b": {
                this.emulator?.set_b(is_up);
                break;
              }
              case "start": {
                this.emulator?.set_start(is_up);
                break;
              }
              case "select": {
                this.emulator?.set_select(is_up);
                break;
              }
              case "left": {
                this.emulator?.set_left(is_up);
                break;
              }
              case "right": {
                this.emulator?.set_right(is_up);
                break;
              }
              case "up": {
                this.emulator?.set_up(is_up);
                break;
              }
              case "down": {
                this.emulator?.set_down(is_up);
                break;
              }
            }
          }
        }
      },
    );
    this.port.start();
    console.log("Requesting wasm");
    this.port.postMessage({ type: "wasm" } satisfies FromNodeMessage);
  }

  process(
    _inputs: Float32Array[][],
    outputs: Float32Array[][],
    _parameters: Record<string, Float32Array>,
  ) {
    const left = outputs[0]?.[0];
    const right = outputs[0]?.[1];

    if (!left || !right) {
      throw new Error("No stereo");
    }

    this.emulator?.drive_and_sample(
      left,
      right,
      sampleRate,
      (frame: Uint8Array) => {
        this.port.postMessage({
          type: "frame",
          bytes: frame,
        } satisfies FromNodeMessage);
      },
    );

    return true;
  }
}

registerProcessor(AUDIO_PROCESSOR_NAME, WasmProcessor);
