//! Linux audio output through ALSA.
//!
//! `libasound.so.2` is opened at runtime with `dlopen`, so building needs no
//! ALSA headers or pkg-config, and a machine without ALSA or without any
//! sound card (CI, containers) just reports the error and runs silently.
//! PulseAudio and PipeWire desktops provide the ALSA "default" device, so
//! this covers them too. A dedicated thread mixes and writes 16-bit stereo.

use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use engine::mixer::MIX_CHANNELS;
use engine::{Mixer, MixerCmd};

use super::output;

const RATE: u32 = 44_100;
const PERIOD_FRAMES: usize = 512;
const LATENCY_US: c_uint = 60_000;
// Values of the ALSA enums used (stable ABI).
const SND_PCM_STREAM_PLAYBACK: c_int = 0;
const SND_PCM_FORMAT_S16_LE: c_int = 2;
const SND_PCM_ACCESS_RW_INTERLEAVED: c_int = 3;

type PcmOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
type PcmSetParams = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
type PcmWritei = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulong) -> c_long;
type PcmRecover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type PcmClose = unsafe extern "C" fn(*mut c_void) -> c_int;
type StrError = unsafe extern "C" fn(c_int) -> *const c_char;

/// The few libasound entry points needed for blocking playback.
struct Lib {
    handle: *mut c_void,
    open: PcmOpen,
    set_params: PcmSetParams,
    writei: PcmWritei,
    recover: PcmRecover,
    close: PcmClose,
    strerror: StrError,
}

impl Lib {
    fn load() -> Result<Self, String> {
        for name in [c"libasound.so.2", c"libasound.so"] {
            // SAFETY: dlopen with a valid C string; a null result is handled.
            let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
            if handle.is_null() {
                continue;
            }
            let sym = |s: &CStr| -> Result<*mut c_void, String> {
                // SAFETY: handle is a live library handle, s a valid C string.
                let p = unsafe { libc::dlsym(handle, s.as_ptr()) };
                if p.is_null() {
                    Err(format!("libasound is missing {}", s.to_string_lossy()))
                } else {
                    Ok(p)
                }
            };
            let resolve = || -> Result<Self, String> {
                // SAFETY: each symbol is the documented libasound function with
                // exactly the signature it is transmuted to.
                unsafe {
                    Ok(Self {
                        handle,
                        open: std::mem::transmute::<*mut c_void, PcmOpen>(sym(c"snd_pcm_open")?),
                        set_params: std::mem::transmute::<*mut c_void, PcmSetParams>(sym(c"snd_pcm_set_params")?),
                        writei: std::mem::transmute::<*mut c_void, PcmWritei>(sym(c"snd_pcm_writei")?),
                        recover: std::mem::transmute::<*mut c_void, PcmRecover>(sym(c"snd_pcm_recover")?),
                        close: std::mem::transmute::<*mut c_void, PcmClose>(sym(c"snd_pcm_close")?),
                        strerror: std::mem::transmute::<*mut c_void, StrError>(sym(c"snd_strerror")?),
                    })
                }
            };
            return resolve().inspect_err(|_| {
                // SAFETY: handle came from dlopen and is not used afterwards.
                unsafe { libc::dlclose(handle) };
            });
        }
        Err("ALSA library (libasound.so.2) not found".into())
    }

    fn error(&self, code: c_int) -> String {
        // SAFETY: snd_strerror returns a static NUL-terminated string.
        unsafe {
            let p = (self.strerror)(code);
            if p.is_null() {
                format!("error {code}")
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for Lib {
    fn drop(&mut self) {
        // SAFETY: handle came from dlopen; no function pointers outlive self.
        unsafe { libc::dlclose(self.handle) };
    }
}

/// Running output; dropping it stops the audio thread.
pub struct Output {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    desc: String,
}

impl Output {
    pub fn describe(&self) -> &str {
        &self.desc
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Open the default ALSA device and start the mixing thread.
pub fn open() -> Result<(Output, Sender<MixerCmd>), String> {
    let (tx, rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let thread = std::thread::Builder::new()
        .name("audio-out".into())
        .spawn(move || run(rx, ready_tx, stop_thread))
        .map_err(|e| format!("cannot start audio thread: {e}"))?;
    match ready_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(desc)) => Ok((Output { stop, thread: Some(thread), desc }, tx)),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => {
            // Leave the stuck thread detached; it exits once it notices.
            stop.store(true, Ordering::Relaxed);
            Err("audio device did not respond".into())
        }
    }
}

fn run(rx: Receiver<MixerCmd>, ready: SyncSender<Result<String, String>>, stop: Arc<AtomicBool>) {
    let lib = match Lib::load() {
        Ok(l) => l,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let mut pcm: *mut c_void = std::ptr::null_mut();
    // SAFETY: valid out-pointer and device name.
    let r = unsafe { (lib.open)(&mut pcm, c"default".as_ptr(), SND_PCM_STREAM_PLAYBACK, 0) };
    if r < 0 || pcm.is_null() {
        let _ = ready.send(Err(format!("cannot open ALSA device \"default\": {}", lib.error(r))));
        return;
    }
    // SAFETY: pcm is an open playback handle.
    let r = unsafe { (lib.set_params)(pcm, SND_PCM_FORMAT_S16_LE, SND_PCM_ACCESS_RW_INTERLEAVED, MIX_CHANNELS as c_uint, RATE, 1, LATENCY_US) };
    if r < 0 {
        let _ = ready.send(Err(format!("cannot configure ALSA device: {}", lib.error(r))));
        // SAFETY: pcm is open and not used afterwards.
        unsafe { (lib.close)(pcm) };
        return;
    }
    let _ = ready.send(Ok(format!("ALSA \"default\" device, {RATE} Hz stereo")));

    let mut mixer = Mixer::new(RATE);
    let mut mixed = vec![0.0f32; PERIOD_FRAMES * MIX_CHANNELS];
    let mut pcm16 = vec![0i16; PERIOD_FRAMES * MIX_CHANNELS];
    'play: while !stop.load(Ordering::Relaxed) {
        if !output::drain(&rx, &mut mixer) {
            break;
        }
        mixer.mix(&mut mixed);
        output::spread(&mixed, &mut pcm16, MIX_CHANNELS, output::to_i16);
        let mut done = 0;
        while done < PERIOD_FRAMES {
            let rest = &pcm16[done * MIX_CHANNELS..];
            // SAFETY: rest holds at least (PERIOD_FRAMES - done) stereo frames.
            let n = unsafe { (lib.writei)(pcm, rest.as_ptr() as *const c_void, (PERIOD_FRAMES - done) as c_ulong) };
            if n < 0 {
                // Underrun or suspend: let ALSA recover and retry.
                // SAFETY: pcm is open.
                let r = unsafe { (lib.recover)(pcm, n as c_int, 1) };
                if r < 0 {
                    eprintln!("audio: ALSA playback failed ({}); continuing without sound", lib.error(r));
                    break 'play;
                }
            } else {
                done += n as usize;
            }
            if stop.load(Ordering::Relaxed) {
                break 'play;
            }
        }
    }
    // SAFETY: pcm is open and not used afterwards.
    unsafe { (lib.close)(pcm) };
}
