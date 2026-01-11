export const AUDIO_PROCESSOR_NAME = "gebeh-audio-processor";
export type FromNodeMsg =
  | {
      type: "ready";
    }
  | { type: "wasm" };
export type FromMainMsg =
  | {
      type: "rom";
      bytes: ArrayBuffer;
    }
  | { type: "wasm"; bytes: ArrayBuffer };
