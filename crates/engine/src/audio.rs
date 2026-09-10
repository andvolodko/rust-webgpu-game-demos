//! Tiny procedural SFX. No sample files — oscillators + noise.
//! WASM: Web Audio. Native: cpal mixer.

use std::sync::{Mutex, OnceLock};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    Paddle,
    Brick,
    Shatter,
    Wall,
    Pickup,
    Laser,
    LifeLost,
    Launch,
    Win,
    Cannon,
    Explosion,
    Spawn,
}

const RATE: f32 = 22050.0;

fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as i32 as f32) / 2_147_483_648.0
}

fn render(sfx: Sfx) -> Vec<f32> {
    match sfx {
        Sfx::Paddle => thud(180.0, 0.07, 0.55),
        Sfx::Brick => tick(920.0, 0.045, 0.4),
        Sfx::Shatter => burst(0.16, 0.5, 420.0),
        Sfx::Wall => tick(640.0, 0.035, 0.28),
        Sfx::Pickup => arpeggio(&[523.0, 659.0, 784.0], 0.07, 0.42),
        Sfx::Laser => laser(0.055, 0.38),
        Sfx::LifeLost => sweep(320.0, 90.0, 0.28, 0.45),
        Sfx::Launch => sweep(220.0, 640.0, 0.12, 0.32),
        Sfx::Win => arpeggio(&[392.0, 494.0, 587.0, 784.0], 0.09, 0.4),
        Sfx::Cannon => boom(0.22, 0.55),
        Sfx::Explosion => burst(0.28, 0.62, 90.0),
        Sfx::Spawn => arpeggio(&[330.0, 440.0], 0.05, 0.3),
    }
}

fn thud(hz: f32, dur: f32, amp: f32) -> Vec<f32> {
    let n = (dur * RATE) as usize;
    let mut o = vec![0.0; n];
    let mut seed = 0xA11C_E5u32;
    for (i, s) in o.iter_mut().enumerate() {
        let t = i as f32 / RATE;
        let env = (1.0 - t / dur).max(0.0).powf(1.6);
        let tone = (std::f32::consts::TAU * hz * t).sin() * 0.7
            + (std::f32::consts::TAU * hz * 0.5 * t).sin() * 0.35;
        *s = (tone + noise(&mut seed) * 0.12) * env * amp;
    }
    o
}

fn tick(hz: f32, dur: f32, amp: f32) -> Vec<f32> {
    let n = (dur * RATE) as usize;
    let mut o = vec![0.0; n];
    for (i, s) in o.iter_mut().enumerate() {
        let t = i as f32 / RATE;
        let env = (1.0 - t / dur).max(0.0).powf(2.4);
        *s = (std::f32::consts::TAU * hz * t).sin() * env * amp;
    }
    o
}

fn sweep(from: f32, to: f32, dur: f32, amp: f32) -> Vec<f32> {
    let n = (dur * RATE) as usize;
    let mut o = vec![0.0; n];
    let mut phase = 0.0;
    for (i, s) in o.iter_mut().enumerate() {
        let u = i as f32 / n.max(1) as f32;
        let hz = from + (to - from) * u;
        phase += std::f32::consts::TAU * hz / RATE;
        let env = (1.0 - u).powf(1.2) * (u * 18.0).min(1.0);
        *s = phase.sin() * env * amp;
    }
    o
}

fn burst(dur: f32, amp: f32, low: f32) -> Vec<f32> {
    let n = (dur * RATE) as usize;
    let mut o = vec![0.0; n];
    let mut seed = 0x51C0_FFEE;
    let mut lp = 0.0;
    for (i, s) in o.iter_mut().enumerate() {
        let u = i as f32 / n.max(1) as f32;
        let env = (1.0 - u).powf(1.8);
        let nse = noise(&mut seed);
        lp = lp * 0.86 + nse * 0.14;
        let boom = (std::f32::consts::TAU * low * (1.0 - u * 0.7) * (i as f32 / RATE)).sin();
        *s = (lp * 0.75 + boom * 0.35) * env * amp;
    }
    o
}

fn boom(dur: f32, amp: f32) -> Vec<f32> {
    burst(dur, amp, 70.0)
}

fn laser(dur: f32, amp: f32) -> Vec<f32> {
    let n = (dur * RATE) as usize;
    let mut o = vec![0.0; n];
    let mut seed = 0x1A5E_0001u32;
    let mut phase = 0.0;
    for (i, s) in o.iter_mut().enumerate() {
        let u = i as f32 / n.max(1) as f32;
        let hz = 1680.0 - 900.0 * u;
        phase += std::f32::consts::TAU * hz / RATE;
        let env = (1.0 - u).powf(1.4) * (u * 30.0).min(1.0);
        *s = (phase.sin() * 0.7 + noise(&mut seed) * 0.22) * env * amp;
    }
    o
}

fn arpeggio(notes: &[f32], note_dur: f32, amp: f32) -> Vec<f32> {
    let mut o = Vec::new();
    for (k, hz) in notes.iter().enumerate() {
        let n = (note_dur * RATE) as usize;
        for i in 0..n {
            let t = i as f32 / RATE;
            let env = (1.0 - t / note_dur).max(0.0).powf(1.5);
            let s = (std::f32::consts::TAU * hz * t).sin() * env * amp * (0.85 + k as f32 * 0.04);
            o.push(s);
        }
    }
    o
}

#[cfg(not(target_arch = "wasm32"))]
struct Voice {
    samples: Vec<f32>,
    pos: usize,
}

#[cfg(not(target_arch = "wasm32"))]
struct Mixer {
    voices: Vec<Voice>,
    out_rate: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Mixer {
    fn new(out_rate: f32) -> Self {
        Self {
            voices: Vec::new(),
            out_rate: out_rate.max(8000.0),
        }
    }

    fn push(&mut self, clip: Vec<f32>) {
        if self.voices.len() >= 16 {
            return;
        }
        let ratio = self.out_rate / RATE;
        let n = ((clip.len() as f32) * ratio).round() as usize;
        let mut samples = vec![0.0; n.max(1)];
        for i in 0..samples.len() {
            let src = i as f32 / ratio.max(0.001);
            let j = src as usize;
            let f = src - j as f32;
            let a = clip.get(j).copied().unwrap_or(0.0);
            let b = clip.get(j + 1).copied().unwrap_or(a);
            samples[i] = a * (1.0 - f) + b * f;
        }
        self.voices.push(Voice { samples, pos: 0 });
    }

    fn pull(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        for v in &mut self.voices {
            let n = (v.samples.len() - v.pos).min(out.len());
            for i in 0..n {
                out[i] = (out[i] + v.samples[v.pos + i]).clamp(-1.0, 1.0);
            }
            v.pos += n;
        }
        self.voices.retain(|v| v.pos < v.samples.len());
    }
}

struct Inner {
    #[cfg(not(target_arch = "wasm32"))]
    mixer: Arc<Mutex<Mixer>>,
    #[cfg(target_arch = "wasm32")]
    ctx: Option<web_sys::AudioContext>,
}

fn inner() -> &'static Mutex<Inner> {
    static I: OnceLock<Mutex<Inner>> = OnceLock::new();
    I.get_or_init(|| Mutex::new(Inner::new()))
}

impl Inner {
    fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mixer = Arc::new(Mutex::new(Mixer::new(44100.0)));
            if let Some(stream) = start_cpal(mixer.clone()) {
                std::mem::forget(stream);
            }
            Self { mixer }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let ctx = web_sys::AudioContext::new().ok();
            Self { ctx }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn start_cpal(mixer: Arc<Mutex<Mixer>>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let cfg = device.default_output_config().ok()?;
    let channels = cfg.channels() as usize;
    let rate = cfg.sample_rate().0 as f32;
    if let Ok(mut m) = mixer.lock() {
        m.out_rate = rate;
    }
    let mix = mixer;
    let err_fn = |e| eprintln!("audio: {e}");
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                &cfg.config(),
                move |data: &mut [f32], _| write_f32(data, channels, &mix),
                err_fn,
                None,
            )
            .ok()?,
        _ => return None,
    };
    stream.play().ok()?;
    Some(stream)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_f32(data: &mut [f32], channels: usize, mix: &Arc<Mutex<Mixer>>) {
    let frames = data.len() / channels.max(1);
    let mut mono = vec![0.0f32; frames];
    if let Ok(mut m) = mix.lock() {
        m.pull(&mut mono);
    }
    for (i, frame) in mono.iter().enumerate() {
        for c in 0..channels {
            if let Some(s) = data.get_mut(i * channels + c) {
                *s = *frame;
            }
        }
    }
}

/// Resume Web Audio after a user gesture (required by browsers).
pub fn resume() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(g) = inner().lock() {
            if let Some(ctx) = &g.ctx {
                let _ = ctx.resume();
            }
        }
    }
}

pub fn play(sfx: Sfx) {
    let clip = render(sfx);
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(g) = inner().lock() {
            if let Ok(mut m) = g.mixer.lock() {
                m.push(clip);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        play_web(&clip);
    }
}

#[cfg(target_arch = "wasm32")]
fn play_web(clip: &[f32]) {
    let Ok(mut g) = inner().lock() else {
        return;
    };
    if g.ctx.is_none() {
        g.ctx = web_sys::AudioContext::new().ok();
    }
    let Some(ctx) = g.ctx.as_ref() else {
        return;
    };
    let _ = ctx.resume();
    let n = clip.len() as u32;
    if n == 0 {
        return;
    }
    let Ok(buf) = ctx.create_buffer(1, n, RATE) else {
        return;
    };
    let mut copy = clip.to_vec();
    if buf.copy_to_channel(&mut copy, 0).is_err() {
        return;
    }
    let Ok(src) = ctx.create_buffer_source() else {
        return;
    };
    src.set_buffer(Some(&buf));
    let Ok(gain) = ctx.create_gain() else {
        return;
    };
    gain.gain().set_value(0.85);
    let _ = src.connect_with_audio_node(&gain);
    let _ = gain.connect_with_audio_node(&ctx.destination());
    let _ = src.start();
}
