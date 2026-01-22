import "../polyfill/TextEncoder.js";
import { initSync, WebEmulator } from "../pkg/gebeh_web.js";
import {
  AUDIO_PROCESSOR_NAME,
  FromMainMessage,
  FromNodeMessage,
  GB_HEIGHT,
  GB_WIDTH,
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
  currentFrame = new Uint8Array(new ArrayBuffer(GB_WIDTH * GB_HEIGHT));
  poor_mans_time = 0;

  constructor() {
    super();
    this.port.addEventListener(
      "message",
      ({ data }: MessageEvent<FromMainMessage>) => {
        switch (data.type) {
          case "rom": {
            this.emulator?.load_rom(
              new Uint8Array(data.bytes),
              data.save ? new Uint8Array(data.save) : undefined,
            );
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
            const is_down = data.event === "down";
            switch (data.button) {
              case "a": {
                this.emulator?.set_a(is_down);
                break;
              }
              case "b": {
                this.emulator?.set_b(is_down);
                break;
              }
              case "start": {
                this.emulator?.set_start(is_down);
                break;
              }
              case "select": {
                this.emulator?.set_select(is_down);
                break;
              }
              case "left": {
                this.emulator?.set_left(is_down);
                break;
              }
              case "right": {
                this.emulator?.set_right(is_down);
                break;
              }
              case "up": {
                this.emulator?.set_up(is_down);
                break;
              }
              case "down": {
                this.emulator?.set_down(is_down);
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

    if (!this.emulator) {
      return true;
    }

    this.emulator.drive_and_sample(
      left,
      right,
      sampleRate,
      () => {
        this.port.postMessage({
          type: "frame",
          buffer: this.currentFrame,
        } satisfies FromNodeMessage);
      },
      this.currentFrame,
    );

    // https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/process
    // Citation: audio data blocks are always 128 frames long
    // loop every ~5 seconds
    this.poor_mans_time =
      (this.poor_mans_time + 1) % Math.round((sampleRate / 128) * 5);

    if (this.poor_mans_time === 0) {
      const save = this.emulator.get_save();
      if (save) {
        const ram = save.get_ram();
        this.port.postMessage(
          {
            type: "save",
            buffer: ram,
            title: save.get_game_title(),
          } satisfies FromNodeMessage,
          [ram.buffer],
        );
      }
    }

    return true;
  }
}

registerProcessor(AUDIO_PROCESSOR_NAME, WasmProcessor);
