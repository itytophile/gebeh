import { initSync, WasmAudioProcessor } from "http://localhost:3000/pkg/gebeh_web.js"

registerProcessor(
  "WasmProcessor",
  class WasmProcessor extends AudioWorkletProcessor {
    constructor(options) {
      super();
      let [module, memory, handle] = options.processorOptions;
      initSync({ module, memory });
      this.processor = WasmAudioProcessor.unpack(handle);
    }
    process(inputs, outputs) {
      const [left, right] = outputs[0];
      return this.processor.process(left, right, sampleRate);
    }
  },
);
