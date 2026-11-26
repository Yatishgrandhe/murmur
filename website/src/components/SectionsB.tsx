import { AnimatePresence, motion } from 'framer-motion';
import { useId, useState } from 'react';
import { PASTE, REPO, type OS } from '../site';
import { DownloadButtons } from './Hero';
import { AppleGlyph, IconChevron, IconMic, WindowsGlyph } from './Icons';
import { Keys, MurmurApp, Pill } from './Mock';
import { MurmurIcon } from './MurmurIcon';
import { Segmented } from './SectionsA';

/* ---------- 6. Your AI (black) ---------- */
const PROVIDERS = {
  claude: { name: 'Claude', runs: 'Claude Code, signed in with your Claude account', pays: 'Nothing extra. Counts toward your Pro or Max usage.', speed: 'About a second' },
  chatgpt: { name: 'ChatGPT', runs: 'OpenAI’s Codex app, signed in with ChatGPT', pays: 'Nothing extra. Counts toward your Plus or Pro usage.', speed: '3 to 4.5 seconds' },
  claudeApi: { name: 'Claude API', runs: 'A direct request with your key, kept in the system keychain', pays: 'Anthropic’s per-token price', speed: 'Depends on the model' },
  openaiApi: { name: 'OpenAI API', runs: 'A direct request with your key, kept in the system keychain', pays: 'OpenAI’s per-token price', speed: 'Depends on the model' },
  none: { name: 'No rewrite', runs: 'Nothing. You get the transcript exactly as it was heard.', pays: 'Nothing.', speed: 'Instant' },
} as const;
type Provider = keyof typeof PROVIDERS;

export function YourAi({ os }: { os: OS }) {
  const [p, setP] = useState<Provider>('claude');
  const info = PROVIDERS[p];
  return (
    <section className="sec sec--black ai" id="ai">
      <div className="wrap ai-inner">
        <h2>Runs on the AI you already pay for</h2>
        <div className="ai-split">
          <MurmurApp page="ai" dark className="ai-app" os={os}>
            <header className="mapp-head"><h3>AI Rewrite</h3><p>Who cleans up what you said.</p></header>
            <Segmented label="AI provider" tone="dark" value={p} onChange={setP} options={(Object.keys(PROVIDERS) as Provider[]).map((k) => ({ id: k, label: PROVIDERS[k].name }))} />
            <AnimatePresence mode="wait" initial={false}>
              <motion.dl key={p} className="ai-rows" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: 0.2 }}>
                <div><dt>What runs</dt><dd>{info.runs}</dd></div>
                <div><dt>What you pay</dt><dd>{info.pays}</dd></div>
                <div><dt>Speed</dt><dd>{info.speed}</dd></div>
              </motion.dl>
            </AnimatePresence>
          </MurmurApp>
          <div className="ai-figure">
            <span className="huge">~1s</span>
            <p>From letting go of the key to text in the box. Measured on an M4 MacBook with Claude Haiku.</p>
          </div>
        </div>
        <p className="ai-quiet">No API key or extra subscription needed: connect Claude Code or Codex once and Murmur uses that. Speech-to-text is built into macOS and Windows, so the app is under 10 MB with nothing else to download. If the AI is down or you hit a limit, you get the plain transcript instead of nothing.</p>
      </div>
    </section>
  );
}

/* ---------- 7. Dictionary + History (light) ---------- */
export function Memory({ os }: { os: OS }) {
  return (
    <section className="sec sec--light memory">
      <div className="wrap memory-inner">
        <div className="memory-copy">
          <h2>It knows how Priya spells it</h2>
          <p className="body">Add names and jargon once. Every dictation is saved on this computer, with what you said next to what got typed.</p>
        </div>
        <div className="memory-wins">
          <MurmurApp page="dictionary" className="mem-app" compact os={os} />
          <MurmurApp page="history" className="mem-app mem-app--history" compact os={os} />
        </div>
      </div>
    </section>
  );
}

/* ---------- 8. Privacy (black) ---------- */
export function Privacy({ os }: { os: OS }) {
  const mac = os === 'mac';
  return (
    <section className="sec sec--black privacy" id="privacy">
      <div className="wrap privacy-inner">
        <h2 className="privacy-line">The AI reads your words. It never hears your voice</h2>
        <div className={`privacy-visual ${mac ? '' : 'privacy-visual--win'}`} aria-hidden>
          <div className="wallpaper wallpaper--night" />
          {mac ? (
            <div className="menubar">
              <AppleGlyph size={16} /><b>Murmur</b><span>File</span><span>Edit</span><span>Window</span>
              <span className="mb-gap" />
              <span className="mb-anchor">
                <span className="mb-dot" />
                <span className="callout callout--down"><span className="callout-line" /><span className="callout-label">Orange dot = mic is on</span></span>
              </span>
              <MurmurIcon size={18} />
              <span>Wed 9:41 AM</span>
            </div>
          ) : (
            <div className="taskbar">
              <WindowsGlyph size={18} />
              <span className="mb-gap" />
              <span className="mb-anchor">
                <span className="tb-mic"><IconMic size={15} /></span>
                <span className="callout callout--up"><span className="callout-label">Mic icon = mic is on</span><span className="callout-line" /></span>
              </span>
              <MurmurIcon size={18} />
              <span>9:41 AM</span>
            </div>
          )}
          <div className="toast">
            <span className="toast-k">Sent to Claude</span>
            <span>“Running about 10 minutes late!”</span>
            <small>The transcript only. No audio.</small>
          </div>
          <div className="privacy-pill"><Pill state="listen" text="um so tell john I’m like running ten min late" level={0.7} /></div>
        </div>
        <ul className="privacy-facts">
          {mac ? (
            <li><b>Speech to text on your Mac.</b> Apple’s speech recognition runs on the Mac itself when your Mac supports on-device recognition.</li>
          ) : (
            <li><b>Speech to text by Microsoft.</b> Windows dictation uses Microsoft’s online speech recognition, so the audio you dictate goes to Microsoft.</li>
          )}
          <li><b>Text to the AI.</b> Only the transcript, never the audio, goes to Claude or ChatGPT for the rewrite.</li>
          {mac ? (
            <li><b>Kept on this Mac.</b> History holds transcripts, never audio. API keys, if you use them, sit in the Keychain.</li>
          ) : (
            <li><b>Wake word stays local.</b> Windows’ offline phrase recognizer listens for “Hey Murmur”, so background audio isn’t uploaded.</li>
          )}
        </ul>
      </div>
    </section>
  );
}

/* ---------- 10. FAQ + final CTA (black) ---------- */
const faq = (os: OS): [string, React.ReactNode][] => [
  ['Is it actually free?', 'Yes, it’s open source under Apache-2.0. With Claude or ChatGPT it rides on the plan you already pay for, with no API key. Turn rewriting off and it costs nothing at all.'],
  ['Is there a voice model to download?', `No. Murmur uses the speech recognition built into ${os === 'mac' ? 'macOS' : 'Windows'}. The app is under 10 MB and nothing else gets installed.`],
  ['Does Murmur see my Claude or ChatGPT password?', 'No. You sign in on claude.ai or chatgpt.com in your own browser, through Claude Code or Codex. Murmur never sees your password or tokens.'],
  ['Does my audio go anywhere?', os === 'mac'
    ? 'On a Mac, speech becomes text on the Mac itself when on-device recognition is available. Claude or ChatGPT only ever get the transcript.'
    : 'Yes, to Microsoft: Windows dictation uses its online speech recognition. Claude or ChatGPT only ever get the transcript. The wake word is checked offline.'],
  ['I lost what I said', <>It’s still on the clipboard. Press <Keys keys={PASTE[os]} /> to paste the last dictation, even if no text field had focus.</>],
  ['The shortcut does nothing', 'Another app probably owns it. Open Murmur › Shortcut, click the keys and press a new combination.'],
  ['Intel Mac? Linux?', 'Not in the prebuilt releases yet. You can build from source.'],
];

export function Finale({ os }: { os: OS }) {
  const [open, setOpen] = useState<number | null>(0);
  const base = useId();
  return (
    <section className="sec sec--black finale">
      <div className="wrap finale-inner">
        <div className="faq">
          <h2>Questions</h2>
          <div className="faq-list">
            {faq(os).map(([q, a], i) => {
              const on = open === i;
              return (
                <div key={q} className={`faq-item ${on ? 'is-open' : ''}`}>
                  <h3>
                    <button type="button" aria-expanded={on} aria-controls={`${base}-${i}`} onClick={() => setOpen(on ? null : i)}>
                      {q}
                      <motion.span animate={{ rotate: on ? 180 : 0 }} transition={{ duration: 0.2 }}><IconChevron /></motion.span>
                    </button>
                  </h3>
                  <motion.div id={`${base}-${i}`} className="faq-a" initial={false} animate={{ height: on ? 'auto' : 0, opacity: on ? 1 : 0 }} transition={{ duration: 0.25 }}>
                    <p>{a}</p>
                  </motion.div>
                </div>
              );
            })}
          </div>
        </div>
        <div className="final">
          <img src="./murmur-logo.png" width={64} height={64} alt="" />
          <h2>Download Murmur</h2>
          <p className="body">Free for macOS and Windows. Under 10 MB.</p>
          <DownloadButtons os={os} />
          <a className="link" href={REPO} target="_blank" rel="noreferrer">Source on GitHub ›</a>
        </div>
      </div>
      <footer className="footer">
        <span>Murmur · Apache-2.0</span>
        <span>Not affiliated with Anthropic or OpenAI.</span>
      </footer>
    </section>
  );
}
