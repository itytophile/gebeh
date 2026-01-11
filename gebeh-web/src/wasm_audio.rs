// https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/wasm-audio-worklet/src/wasm_audio.rs

use core::panic::AssertUnwindSafe;
use core::pin::Pin;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys;
use web_sys::{AudioContext, AudioWorkletNode, AudioWorkletNodeOptions};

type ProcessFunction = Box<dyn FnMut(&mut [f32], &mut [f32], f32) -> bool>;

#[wasm_bindgen]
pub struct WasmAudioProcessor(ProcessFunction);

#[wasm_bindgen]
impl WasmAudioProcessor {
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], sample_rate: f32) -> bool {
        self.0(left, right, sample_rate)
    }
    pub fn pack(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }
    pub unsafe fn unpack(val: usize) -> Self {
        *Box::from_raw(val as *mut _)
    }
}

// Use wasm_audio if you have a single Wasm audio processor in your application
// whose samples should be played directly. Ideally, call wasm_audio based on
// user interaction. Otherwise, resume the context on user interaction, so
// playback starts reliably on all browsers.
#[allow(clippy::type_complexity)]
#[must_use]
pub fn wasm_audio(
    process: ProcessFunction,
) -> AssertUnwindSafe<Pin<Box<dyn std::future::Future<Output = Result<AudioContext, JsValue>>>>> {
    let process = AssertUnwindSafe(process);
    AssertUnwindSafe(Box::pin(async {
        let ctx = AudioContext::new()?;
        prepare_wasm_audio(&ctx).await?;
        let node = wasm_audio_node(&ctx, process.0)?;
        node.connect_with_audio_node(&ctx.destination())?;
        Ok(ctx)
    }))
}

// wasm_audio_node creates an AudioWorkletNode running a Wasm audio processor.
// Remember to call prepare_wasm_audio once on your context before calling
// this function.
pub fn wasm_audio_node(
    ctx: &AudioContext,
    process: ProcessFunction,
) -> Result<AudioWorkletNode, JsValue> {
    let options = AudioWorkletNodeOptions::new();
    options.set_processor_options(Some(&js_sys::Array::of3(
        &wasm_bindgen::module(),
        &wasm_bindgen::memory(),
        &WasmAudioProcessor(process).pack().into(),
    )));
    // stereo
    options.set_output_channel_count(&JsValue::from(js_sys::Array::of1(&JsValue::from_f64(2.))));
    AudioWorkletNode::new_with_options(ctx, "WasmProcessor", &options)
}

pub async fn prepare_wasm_audio(ctx: &AudioContext) -> Result<(), JsValue> {
    let mod_url = wasm_bindgen::link_to!(module = "/src/worklet.js");
    JsFuture::from(ctx.audio_worklet()?.add_module(&mod_url)?).await?;
    Ok(())
}
