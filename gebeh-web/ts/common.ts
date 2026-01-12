export const AUDIO_PROCESSOR_NAME = "gebeh-audio-processor";
export type FromNodeMessage =
  | {
      type: "ready";
    }
  | { type: "wasm" }
  | { type: "frame"; bytes: Uint8Array };
export type FromMainMessage =
  | {
      type: "rom";
      bytes: ArrayBuffer;
    }
  | { type: "wasm"; bytes: ArrayBuffer };
