export const AUDIO_PROCESSOR_NAME = "gebeh-audio-processor";
export type FromNodeMessage =
  | {
      type: "ready";
    }
  | { type: "wasm" }
  | { type: "frame"; buffer: ArrayBuffer }
  | { type: "save"; buffer: ArrayBuffer; title: string };
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
      save?: ArrayBuffer;
    }
  | { type: "wasm"; bytes: ArrayBuffer }
  | {
      type: "input";
      event: "up" | "down";
      button: GebehButton;
    };
export const GB_WIDTH = 160;
export const GB_HEIGHT = 144;
