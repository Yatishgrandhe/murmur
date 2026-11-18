import { AnimatePresence, motion } from 'framer-motion';
import { useId, useState } from 'react';
import { REPO, type OS } from '../site';
import { DownloadButtons } from './Hero';
import { AppleGlyph, IconCheck, IconChevron, IconCopy, IconMic, WindowsGlyph } from './Icons';
import { MurmurApp, Pill } from './Mock';
import { MurmurIcon } from './MurmurIcon';
import { useInstaller } from '../useInstaller';
import { Segmented } from './SectionsA';

/* ---------- 6. Your AI (black) ---------- */
const PROVIDERS = {
  claude: { name: 'Claude', runs: 'Claude Code, signed in with your Claude account', pays: 'Nothing extra. Counts toward your Pro or Max usage.', speed: 'About a second' },
  chatgpt: { name: 'ChatGPT', runs: 'OpenAI’s Codex app, signed in with ChatGPT', pays: 'Nothing extra. Counts toward your Plus or Pro usage.', speed: '3 to 4.5 seconds' },
  claudeApi: { name: 'Claude API', runs: 'A direct request with your key, kept in the system keychain', pays: 'Anthropic’s per-token price', speed: 'Depends on the model' },
  openaiApi: { name: 'OpenAI API', runs: 'A direct request with your key, kept in the system keychain', pays: 'OpenAI’s per-token price', speed: 'Depends on the model' },
  none: { name: 'No rewrite', runs: 'Nothing. You get Whisper’s transcript as is.', pays: 'Nothing. Works offline.', speed: 'Instant' },
} as const;
type Provider = keyof typeof PROVIDERS;

export function YourAi() {
  const [p, setP] = useState<Provider>('claude');
  const info = PROVIDERS[p];
  return (
    <section className="sec sec--black ai" id="ai">
      <div className="wrap ai-inner">
        <h2>Runs on the AI you already pay for</h2>
        <div className="ai-split">
          <MurmurApp page="ai" dark className="ai-app">
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
        <p className="ai-quiet">The app is about 10 MB. The voice model is 75–550 MB, downloaded once. If the AI is down or you hit a limit, you get your raw transcript instead of nothing.</p>
      </div>
    </section>
  );
}

/* ---------- 7. Dictionary + History (light) ---------- */
const WORDS = ['Priya', 'Kubernetes', 'Anaïs', 'PostgreSQL', 'Tauri', 'GPT-5', 'Okonkwo', 'Figma', 'Wednesday standup'];
const HISTORY = [
  { t: '9:41 AM', said: 'um so tell john I’m like running ten min late', wrote: 'Running about 10 minutes late!', ms: '0.8 s' },
  { t: '9:12 AM', said: 'can you ask priya if the kubernetes thing is uh fixed', wrote: 'Can you ask Priya if the Kubernetes issue is fixed?', ms: '0.9 s' },
  { t: 'Yesterday', said: 'note to self buy oat milk eggs and the good coffee', wrote: 'Buy oat milk, eggs, and the good coffee.', ms: '0.7 s' },
  { t: 'Yesterday', said: 'tuesday no wait wednesday at three', wrote: 'Wednesday at 3.', ms: '0.6 s' },
];

export function Memory() {
  return (
    <section className="sec sec--light memory">
      <div className="wrap memory-inner">
        <div className="memory-copy">
          <h2>It knows how Priya spells it</h2>
          <p className="body">Add names and jargon once. Every dictation is saved on this computer, with what you said next to what got typed.</p>
        </div>
        <div className="memory-wins">
          <MurmurApp page="dictionary" className="mem-app" compact>
            <header className="mapp-head"><h3>Dictionary</h3><p>Names, brands and jargon Murmur should always get right.</p></header>
            <div className="mapp-card">
              <div className="chips">{WORDS.map((w) => <span key={w} className="chip">{w}<i aria-hidden>×</i></span>)}</div>
              <div className="field">Add a word and press Return</div>
            </div>
          </MurmurApp>
          <MurmurApp page="history" className="mem-app mem-app--history" compact>
            <header className="mapp-head mapp-head--split"><div><h3>History</h3><p>Stored only on this computer.</p></div><span className="ghost-btn">Clear all</span></header>
            <ul className="hist">
              {HISTORY.map((h, i) => (
                <li key={i}>
                  <div className="hist-meta"><span>{h.t}</span><span>{h.ms}</span></div>
                  <p className="hist-wrote">{h.wrote}</p>
                  <p className="hist-said">You said: {h.said}</p>
                </li>
              ))}
            </ul>
          </MurmurApp>
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
        <h2 className="privacy-line">Your voice stays on your computer until you say so</h2>
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
              <span>9:41 AM</span>
            </div>
          )}
          <div className="toast">
            <span className="toast-k">Sent to Claude</span>
            <span>“Running about 10 minutes late!”</span>
            <small>Text only. The audio never left this computer.</small>
          </div>
          <div className="privacy-pill"><Pill state="listen" text="um so tell john I’m like running ten min late" level={0.7} /></div>
        </div>
        <ul className="privacy-facts">
          <li>Whisper turns speech into text on your computer. Only that text goes to the AI you picked.</li>
          <li>The wake word is checked locally. Nothing leaves until it hears the phrase.</li>
          <li>API keys sit in the system keychain. The log keeps transcripts, never audio.</li>
        </ul>
      </div>
    </section>
  );
}

/* ---------- 9. Install (white) ---------- */
function useCopy() {
  const [copied, setCopied] = useState<string | null>(null);
  const copy = async (key: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(key);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      /* clipboard blocked; the command is selectable */
    }
  };
  return { copied, copy };
}

const CMDS: Record<OS, string[]> = {
  mac: ['git clone https://github.com/Yatishgrandhe/murmur.git', 'cd murmur', './scripts/install-mac.sh'],
  windows: ['git clone https://github.com/Yatishgrandhe/murmur.git', 'cd murmur', 'powershell -ExecutionPolicy Bypass -File scripts\\install-windows.ps1'],
};

export function Install({ os: detected }: { os: OS }) {
  const [os, setOs] = useState<OS>(detected);
  const { copied, copy } = useCopy();
  const mac = os === 'mac';
  const exe = useInstaller();
  return (
    <section className="sec sec--white install" id="install">
      <div className="wrap install-inner">
        <div className="install-head">
          <h2>Install it</h2>
          <p className="body">{mac ? (exe.dmg ? 'Download the app for Apple Silicon Macs, or build it yourself with three commands.' : 'There’s no Mac download yet, so you build it. Three commands, a few minutes.') : 'Run the installer, or build it yourself with three commands.'}</p>
        </div>
        <div className="install-main">
          <Segmented label="Operating system" value={os} onChange={setOs} options={[{ id: 'mac', label: 'macOS' }, { id: 'windows', label: 'Windows' }]} />
          <AnimatePresence mode="wait" initial={false}>
            <motion.div key={os} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: 0.2 }}>
              <div className={`term term--${os}`}>
                <div className="term-bar">
                  {mac ? <span className="lights" aria-hidden><i /><i /><i /></span> : <span className="ps-icon" aria-hidden>&gt;_</span>}
                  <span className="term-title">{mac ? 'murmur — zsh' : 'Windows PowerShell'}</span>
                  <button type="button" className="copy-all" onClick={() => copy('all', CMDS[os].join(mac ? ' && ' : '; '))}>
                    {copied === 'all' ? 'Copied' : 'Copy all'}
                  </button>
                </div>
                <div className="term-body">
                  {CMDS[os].map((c, i) => (
                    <div key={c} className="term-line">
                      <span className="term-prompt" aria-hidden>{mac ? '%' : 'PS>'}</span>
                      <code>{c}</code>
                      <button type="button" className="copy-one" onClick={() => copy(`c${i}`, c)} aria-label={copied === `c${i}` ? 'Copied' : `Copy: ${c}`}>
                        {copied === `c${i}` ? <><IconCheck size={14} /> Copied</> : <IconCopy size={15} />}
                      </button>
                    </div>
                  ))}
                  <p className="term-out">{mac ? 'Murmur installed and running. Tap ⌥ Space, speak, and pause.' : 'Builds the installer, installs it for your user, and opens Murmur.'}</p>
                </div>
              </div>
              <dl className="install-facts">
                <div><dt>You need</dt><dd>{mac ? 'Rust, Node.js and the Xcode command-line tools' : 'Rust, Node.js, VS C++ build tools, CMake, LLVM, WebView2'}</dd></div>
                <div><dt>Allow</dt><dd>{mac ? 'Microphone, and Accessibility so it can paste' : 'Microphone, under Settings › Privacy'}</dd></div>
                <div><dt>Then</dt><dd>{mac ? <>Hold <b>⌥ Space</b> and talk.{exe.dmg && <> Or skip building: <a className="link" href={exe.dmg.url}>get the {exe.tag} .dmg, {exe.dmg.size} ›</a> (first launch: right-click › Open)</>}</> : <>Hold <b>Ctrl + Shift + Space</b> and talk. Or skip building: <a className="link" href={exe.url}>get the {exe.tag} installer, {exe.size} ›</a></>}</dd></div>
              </dl>
            </motion.div>
          </AnimatePresence>
        </div>
      </div>
    </section>
  );
}

/* ---------- 10. FAQ + final CTA (black) ---------- */
const FAQ: [string, string][] = [
  ['Is it actually free?', 'Yes, it’s open source under Apache-2.0. With Claude or ChatGPT it rides on the plan you already pay for. With an API key you pay the provider per token. No-rewrite mode costs nothing and works offline.'],
  ['Do I need Claude or ChatGPT?', 'No. Use an API key instead, or switch rewriting off and get the plain transcript.'],
  ['Does my audio go anywhere?', 'No. Whisper runs on your computer. Only the transcript is sent, and only to the AI you picked.'],
  ['The shortcut does nothing', 'Another app probably owns it. Open Shortcut in Murmur, click the keys and press a new combination.'],
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
            {FAQ.map(([q, a], i) => {
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
          <p className="body">Free for macOS and Windows.</p>
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
