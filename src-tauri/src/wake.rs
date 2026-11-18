//! Wake word ("Hey Murmur"). Fully on-device: a cheap loudness segmenter finds short bursts of
//! speech, and only those clips are run through the already-loaded Whisper model and matched
//! against the wake phrase. Nothing leaves the computer unless the phrase is heard.

/// Cuts a continuous 16 kHz stream into speech utterances.
pub struct Segmenter {
    buf: Vec<f32>,
    in_speech: bool,
    silence: usize,
    floor: f32,
}

const RATE: usize = 16_000;
const FRAME: usize = 480; // 30 ms
const END_SILENCE: usize = RATE * 7 / 10; // 0.7 s pause ends an utterance
const MAX_LEN: usize = RATE * 8; // longest clip we bother transcribing
const MIN_LEN: usize = RATE * 4 / 10; // "Hey Murmur" is ~0.6 s
const PRE_ROLL: usize = RATE * 3 / 10; // keep a little audio from before speech started

impl Default for Segmenter {
    fn default() -> Self {
        Self::new()
    }
}

impl Segmenter {
    pub fn new() -> Self {
        Segmenter { buf: Vec::new(), in_speech: false, silence: 0, floor: f32::MAX }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.in_speech = false;
        self.silence = 0;
    }

    /// Feed 16 kHz mono samples; returns finished utterances.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Vec<f32>> {
        let mut out = Vec::new();
        for frame in samples.chunks(FRAME) {
            let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
            if rms < self.floor {
                self.floor = rms;
            } else {
                self.floor += (rms - self.floor) * 0.002;
            }
            let loud = rms > (self.floor * 3.0).max(0.004);
            self.buf.extend_from_slice(frame);
            if self.in_speech {
                if loud {
                    self.silence = 0;
                } else {
                    self.silence += frame.len();
                }
                let done = self.silence >= END_SILENCE || self.buf.len() >= MAX_LEN;
                if done {
                    let clip = std::mem::take(&mut self.buf);
                    let speech_len = clip.len().saturating_sub(self.silence);
                    if speech_len >= MIN_LEN && self.buf.len() < MAX_LEN {
                        out.push(clip);
                    }
                    self.in_speech = false;
                    self.silence = 0;
                }
            } else if loud {
                self.in_speech = true;
                self.silence = 0;
                let keep = self.buf.len().min(PRE_ROLL + frame.len());
                self.buf.drain(..self.buf.len() - keep);
            } else if self.buf.len() > PRE_ROLL {
                self.buf.drain(..self.buf.len() - PRE_ROLL);
            }
        }
        out
    }
}

/// Words Whisper commonly hears instead of "hey" / the app name.
fn normalize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn similar(word: &str, target: &str) -> bool {
    let allowed = if target.len() <= 3 { 1 } else if target.len() <= 6 { 2 } else { 3 };
    edit_distance(word, target) <= allowed
}

const GREETINGS: [&str; 6] = ["hey", "hi", "okay", "ok", "a", "hay"];

/// If the transcript starts with the wake phrase, returns what was said after it ("" if nothing).
pub fn match_wake(transcript: &str, phrase: &str) -> Option<String> {
    let words = normalize(transcript);
    let target = normalize(phrase);
    if target.is_empty() || words.is_empty() {
        return None;
    }
    // the name is the last word of the phrase ("hey murmur" → "murmur"); greetings are fuzzy
    let name = target.last().unwrap();
    let has_greeting = target.len() > 1;
    // the phrase must open the utterance: "<greeting> <name> …" or just "<name> …"
    let i = if has_greeting { 1 } else { 0 };
    if let Some(w) = words.get(i).filter(|w| similar(w, name)) {
        let _ = w;
        if has_greeting && !(GREETINGS.contains(&words[0].as_str()) || similar(&words[0], &target[0])) {
            return None;
        }
        // the remainder, taken from the original text so punctuation/case survive
        let mut seen = 0;
        let mut cut = transcript.len();
        let mut in_word = false;
        for (idx, ch) in transcript.char_indices() {
            let is_w = ch.is_alphanumeric() || ch == '\'';
            if is_w && !in_word {
                in_word = true;
            } else if !is_w && in_word {
                in_word = false;
                seen += 1;
                if seen == i + 1 {
                    cut = idx;
                    break;
                }
            }
        }
        let rest = transcript[cut..].trim_start_matches(|c: char| !c.is_alphanumeric()).trim();
        return Some(rest.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_plain_and_misheard() {
        for t in ["Hey Murmur.", "hey murmur", "Hey, Mermer!", "Hey Murmer", "Okay Murmur", "A murmur."] {
            assert_eq!(match_wake(t, "Hey Murmur").as_deref(), Some(""), "{t}");
        }
    }

    #[test]
    fn returns_the_command() {
        assert_eq!(match_wake("Hey Murmur, tell John I'm running late.", "Hey Murmur").as_deref(), Some("tell John I'm running late."));
        assert_eq!(match_wake("hey mermer send the report", "Hey Murmur").as_deref(), Some("send the report"));
    }

    #[test]
    fn ignores_other_speech() {
        for t in ["I heard a murmur in the hall", "Hey Maria", "Thank you.", "murmurs of the crowd were loud", "Hello there"] {
            assert_eq!(match_wake(t, "Hey Murmur"), None, "{t}");
        }
    }

    #[test]
    fn custom_phrase() {
        assert_eq!(match_wake("Hey Jarvis, open notes", "Hey Jarvis").as_deref(), Some("open notes"));
        assert_eq!(match_wake("Computer, new note", "Computer").as_deref(), Some("new note"));
    }

    #[test]
    fn segmenter_finds_utterance() {
        let mut s = Segmenter::new();
        let quiet = vec![0.0005f32; RATE];
        let speech: Vec<f32> = (0..RATE).map(|i| 0.05 * (i as f32 * 0.07).sin()).collect();
        assert!(s.push(&quiet).is_empty());
        assert!(s.push(&speech).is_empty());
        let got = s.push(&quiet);
        assert_eq!(got.len(), 1);
        assert!(got[0].len() >= RATE && got[0].len() < RATE * 2 + PRE_ROLL);
        // a click is not an utterance
        let mut click = vec![0.0005f32; RATE];
        click[100..400].iter_mut().for_each(|x| *x = 0.3);
        assert!(s.push(&click).is_empty() && s.push(&quiet).is_empty());
    }
}
