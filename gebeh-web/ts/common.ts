export const AUDIO_PROCESSOR_NAME = "gebeh-audio-processor";
export type FromNodeMessage =
  | {
      type: "ready";
    }
  | { type: "wasm" }
  | { type: "frame"; buffer: Uint8Array }
  | { type: "save"; buffer: Uint8Array; title: string };
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
      bytes: Uint8Array;
      save?: Uint8Array;
    }
  | { type: "wasm"; bytes: Uint8Array }
  | {
      type: "input";
      event: "up" | "down";
      button: GebehButton;
    }
  | { type: "disableMessages" }
  | { type: "enableMessages" };
export const GB_WIDTH = 160;
export const GB_HEIGHT = 144;
