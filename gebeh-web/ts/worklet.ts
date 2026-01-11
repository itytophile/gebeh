import {
  initSync,
  SyncInitInput,
  WasmAudioProcessor,
} from "../pkg/gebeh_web.js";

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

registerProcessor(
  "WasmProcessor",
  class WasmProcessor
    extends AudioWorkletProcessor
    implements AudioWorkletProcessorImpl
  {
    processor: WasmAudioProcessor;
    constructor(options: {
      processorOptions: [SyncInitInput, WebAssembly.Memory, number];
    }) {
      super();
      const [module, memory, handle] = options.processorOptions;
      initSync({ module, memory });
      this.processor = WasmAudioProcessor.unpack(handle);
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

      return this.processor.process(left, right, sampleRate);
    }
  },
);
