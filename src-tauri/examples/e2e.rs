//! End-to-end check without the UI: WAV file → Whisper → AI rewrite.
//! Usage: cargo run --release --example e2e -- speech.wav [provider] [style]
use murmur_lib::{audio, llm, settings::Settings, stt};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("wav path");
    let mut s = Settings::default();
    if let Some(p) = args.get(2) { s.provider = p.clone(); }
    if let Some(st) = args.get(3) { s.style = st.clone(); }
    if let Ok(d) = std::env::var("MURMUR_DICT") { s.dictionary = d.split(',').map(String::from).collect(); }

    let mut r = hound::WavReader::open(path).expect("open wav");
    let spec = r.spec();
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().map(|x| x.unwrap()).collect(),
        hound::SampleFormat::Int => r.samples::<i32>().map(|x| x.unwrap() as f32 / (1i64 << (spec.bits_per_sample - 1)) as f32).collect(),
    };
    let mono: Vec<f32> = raw.chunks(spec.channels as usize).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect();
    let pcm = audio::resample_to_16k(&mono, spec.sample_rate);
    println!("audio: {:.2}s @ {} Hz → 16 kHz, speech={}", mono.len() as f32 / spec.sample_rate as f32, spec.sample_rate, audio::has_speech(&pcm));

    // shortcut pressed: warm up the AI while "speaking"
    let t_press = Instant::now();
    let pending = llm::Pending::start(&s).expect("start provider");
    let stt = stt::Stt::new();
    stt.ensure_loaded(&s.model).expect("load model");
    println!("model load: {} ms", t_press.elapsed().as_millis());
    std::thread::sleep(std::time::Duration::from_secs_f32((mono.len() as f32 / spec.sample_rate as f32).min(4.0)));

    // shortcut released
    let t = Instant::now();
    let text = llm::apply_voice_commands(&stt.transcribe(&s.model, &pcm, &s.language, &s.dictionary).expect("transcribe"));
    let t_stt = t.elapsed().as_millis();
    let t2 = Instant::now();
    stt.transcribe(&s.model, &pcm, &s.language, &s.dictionary).expect("second pass");
    println!("whisper first {t_stt} ms, warm {} ms: {text}", t2.elapsed().as_millis());
    let t = Instant::now();
    let out = match llm::trivial(&text) {
        Some(x) => { pending.cancel(); x }
        None => pending.finish(&text).unwrap_or_else(|e| format!("[rewrite failed: {e}] {text}")),
    };
    println!("{} total after release ({} ms): {out}", llm::provider_label(&s.provider), t.elapsed().as_millis());
}
