export const AUDIO_PROCESSOR_NAME = "gebeh-audio-processor";
export type FromNodeMessage =
  | {
      type: "ready";
    }
  | { type: "wasm" };
export type FromMainMessage =
  | {
      type: "rom";
      bytes: ArrayBuffer;
    }
  | { type: "wasm"; bytes: ArrayBuffer };
