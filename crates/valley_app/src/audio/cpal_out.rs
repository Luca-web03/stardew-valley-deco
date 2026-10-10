//! Windows / macOS audio output through cpal (WASAPI / CoreAudio). The
//! mixer runs inside cpal's data callback.

use std::sync::mpsc::{self, Receiver, Sender};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use engine::mixer::MIX_CHANNELS;
use engine::{Mixer, MixerCmd};

use super::output;

/// Running output; dropping it stops the stream.
pub struct Output {
    _stream: cpal::Stream,
    desc: String,
}

impl Output {
    pub fn describe(&self) -> &str {
        &self.desc
    }
}

struct Callback {
    mixer: Mixer,
    rx: Receiver<MixerCmd>,
    stereo: Vec<f32>,
    channels: usize,
}

impl Callback {
    fn fill<T: Copy>(&mut self, out: &mut [T], conv: impl Fn(f32) -> T) {
        let frames = out.len() / self.channels.max(1);
        self.stereo.resize(frames * MIX_CHANNELS, 0.0);
        output::drain(&self.rx, &mut self.mixer);
        self.mixer.mix(&mut self.stereo);
        output::spread(&self.stereo, out, self.channels, conv);
    }
}

/// Open the default output device and start the stream.
pub fn open() -> Result<(Output, Sender<MixerCmd>), String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| "no default output device".to_string())?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let (tx, rx) = mpsc::channel();
    let mut cb = Callback { mixer: Mixer::new(rate), rx, stereo: Vec::new(), channels };
    let on_error = |e: cpal::StreamError| eprintln!("audio: stream error: {e}");
    let stream = match format {
        cpal::SampleFormat::F32 => device.build_output_stream(&config, move |d: &mut [f32], _: &cpal::OutputCallbackInfo| cb.fill(d, |s| s), on_error, None),
        cpal::SampleFormat::I16 => device.build_output_stream(&config, move |d: &mut [i16], _: &cpal::OutputCallbackInfo| cb.fill(d, output::to_i16), on_error, None),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config,
            move |d: &mut [u16], _: &cpal::OutputCallbackInfo| cb.fill(d, |s| ((s.clamp(-1.0, 1.0) * 0.5 + 0.5) * 65535.0) as u16),
            on_error,
            None,
        ),
        other => return Err(format!("unsupported device sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let name = device.name().unwrap_or_else(|_| "default device".into());
    Ok((Output { _stream: stream, desc: format!("{name}, {rate} Hz, {channels} ch") }, tx))
}
