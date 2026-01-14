export const AUDIO_PROCESSOR_NAME = "gebeh-audio-processor";
export type FromNodeMessage =
  | {
      type: "ready";
    }
  | { type: "wasm" }
  | { type: "frame"; bytes: Uint8Array };
export type GebehButton =
  | "a"
  | "b"
  | "start"
  | "select"
  | "left"
  | "right"
  | "up"
  | "down";
export type FromMainMessage =
  | {
      type: "rom";
      bytes: ArrayBuffer;
    }
  | { type: "wasm"; bytes: ArrayBuffer }
  | {
      type: "input";
      event: "up" | "down";
      button: GebehButton;
    };
