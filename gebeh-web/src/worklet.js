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
      console.log(outputs);
      return this.processor.process(outputs[0][0]);
    }
  },
);
