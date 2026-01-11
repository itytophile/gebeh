class GebehAudioProcessor extends AudioWorkletProcessor {
  sampleIndex = 0;
  playingSampler;

  constructor() {
    super();
    this.port.onmessage = (sampler) => {
      this.playingSampler = sampler.data;
    };
  }

  process(_inputs, outputs, _parameters) {
    console.log(this.playingSampler);
    if (!this.playingSampler) {
      return true;
    }
    
    console.log(this.playingSampler);

    const [left, right] = outputs[0];

    for (let i = 0; i < left.length; ++i) {
      const sample = this.sampleIndex / sampleRate;
      left[i] = this.playingSampler.sample_left(sample);
      if (right) {
        right[i] = this.playingSampler.sample_right(sample);
      }

      // 10 minutes without pop
      this.sampleIndex = (this.sampleIndex + 1) % (10 * 60 * sampleRate);
    }

    return true;
  }
}

registerProcessor("gebeh-processor", GebehAudioProcessor);
