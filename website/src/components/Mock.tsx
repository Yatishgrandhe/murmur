import { useState, type CSSProperties, type ReactNode } from 'react';
import { KEY_NAME, SHORTCUT, type OS } from '../site';
import { IconBook, IconClock, IconKey, IconSparkle, IconWave } from './Icons';
import { MurmurIcon } from './MurmurIcon';
import { Waveform } from './Waveform';
import { useOS } from '../os';

/* ---------- Window chrome: traffic lights on macOS, caption buttons on Windows ----------
   Pictures only: like the Mac lights, the Windows buttons react to hover and press but don't act. */
export function WindowControls({ os }: { os: OS }) {
  if (os === 'mac') return <span className="lights" aria-hidden><i /><i /><i /></span>;
  return (
    <span className="caps" aria-hidden>
      <span className="cap cap--min"><svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 5.5h10" /></svg></span>
      <span className="cap cap--max"><svg viewBox="0 0 10 10" width="10" height="10"><rect x="0.5" y="0.5" width="9" height="9" rx="1" /></svg></span>
      <span className="cap cap--close"><svg viewBox="0 0 10 10" width="10" height="10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" /></svg></span>
    </span>
  );
}

export function MacWindow({ title, children, className = '', dark = false, style }: { title?: ReactNode; children: ReactNode; className?: string; dark?: boolean; style?: CSSProperties }) {
  const os = useOS();
  return (
    <div className={`win win--${os} ${dark ? 'win--dark' : ''} ${className}`} style={style}>
      <div className="win-bar">
        {os === 'mac' && <WindowControls os="mac" />}
        {title && <span className="win-title">{title}</span>}
        {os === 'windows' && <WindowControls os="windows" />}
      </div>
      <div className="win-body">{children}</div>
    </div>
  );
}

/* ---------- the floating listening pill ---------- */
export type PillState = 'listen' | 'polish' | 'done';
export function Pill({ state, text, level = 0.9, who = 'Claude' }: { state: PillState; text: string; level?: number; who?: string }) {
  return (
    <div className={`pill pill--${state}`} aria-hidden>
      {state === 'listen' && (
        <>
          <span className="pill-rec" />
          <Waveform level={level} bars={18} color="#ffffff" className="pill-wave" />
          <span className="pill-text pill-text--raw">{text}</span>
        </>
      )}
      {state === 'polish' && (
        <>
          <span className="pill-spark"><IconSparkle size={16} /></span>
          <span className="pill-text shimmer">Polishing</span>
          <span className="pill-sub">with {who}</span>
        </>
      )}
      {state === 'done' && (
        <>
          <span className="pill-check">
            <svg viewBox="0 0 14 14" width="12" height="12"><path d="M3 7.4 5.9 10 11 4" fill="none" stroke="#fff" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" /></svg>
          </span>
          <span className="pill-text">{text}</span>
        </>
      )}
    </div>
  );
}

/* ---------- Murmur's own settings window: a small working copy of the app ---------- */
const NAV = [
  { id: 'ai', label: 'AI Rewrite', icon: <IconSparkle size={15} /> },
  { id: 'shortcut', label: 'Shortcut', icon: <IconKey size={15} /> },
  { id: 'voice', label: 'Voice', icon: <IconWave size={15} /> },
  { id: 'dictionary', label: 'Dictionary', icon: <IconBook size={15} /> },
  { id: 'history', label: 'History', icon: <IconClock size={15} /> },
] as const;
export type AppPage = (typeof NAV)[number]['id'];

export const WORDS = ['Priya', 'Kubernetes', 'Anaïs', 'PostgreSQL', 'Tauri', 'GPT-5', 'Okonkwo', 'Figma', 'Wednesday standup'];
export const HISTORY = [
  { t: '9:41 AM', said: 'um so tell john I’m like running ten min late', wrote: 'Running about 10 minutes late!', ms: '0.8 s' },
  { t: '9:12 AM', said: 'can you ask priya if the kubernetes thing is uh fixed', wrote: 'Can you ask Priya if the Kubernetes issue is fixed?', ms: '0.9 s' },
  { t: 'Yesterday', said: 'note to self buy oat milk eggs and the good coffee', wrote: 'Buy oat milk, eggs, and the good coffee.', ms: '0.7 s' },
  { t: 'Yesterday', said: 'tuesday no wait wednesday at three', wrote: 'Wednesday at 3.', ms: '0.6 s' },
];

function DefaultPage({ id, os }: { id: AppPage; os: OS }) {
  switch (id) {
    case 'ai':
      return (
        <>
          <header className="mapp-head"><h3>AI Rewrite</h3><p>Who cleans up what you said.</p></header>
          <dl className="mapp-card mapp-rows">
            <div><dt>Provider</dt><dd>Claude, through Claude Code</dd></div>
            <div><dt>Style</dt><dd>Clean</dd></div>
          </dl>
        </>
      );
    case 'shortcut':
      return (
        <>
          <header className="mapp-head"><h3>Shortcut</h3><p>Hold to talk. Tap once for hands-free.</p></header>
          <div className="mapp-card mapp-rows">
            <div><dt>Talk</dt><dd><Keys keys={SHORTCUT[os]} /></dd></div>
            <div><dt>Cancel</dt><dd><Keys keys={['Esc']} /></dd></div>
          </div>
        </>
      );
    case 'voice':
      return (
        <>
          <header className="mapp-head"><h3>Voice</h3><p>Murmur uses the speech recognition built into your computer. Nothing to download.</p></header>
          <div className="mapp-card mapp-rows">
            <div><dt>Wake word</dt><dd>“Hey Murmur”, off until you turn it on</dd></div>
          </div>
        </>
      );
    case 'dictionary':
      return (
        <>
          <header className="mapp-head"><h3>Dictionary</h3><p>Names, brands and jargon Murmur should always get right.</p></header>
          <div className="mapp-card">
            <div className="chips">{WORDS.map((w) => <span key={w} className="chip">{w}<i aria-hidden>×</i></span>)}</div>
            <div className="field">Add a word and press Return</div>
          </div>
        </>
      );
    case 'history':
      return (
        <>
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
        </>
      );
  }
}

/** Every sidebar row is a full-width button; `children`, when given, replaces the default content of `page`. */
export function MurmurApp({ page, children, dark = false, className = '', compact = false, os = 'mac' }: { page: AppPage; children?: ReactNode; dark?: boolean; className?: string; compact?: boolean; os?: OS }) {
  const [cur, setCur] = useState<AppPage>(page);
  const [moved, setMoved] = useState(false);
  const chrome = useOS();
  return (
    <div className={`mapp mapp--${chrome} ${dark ? 'mapp--dark' : ''} ${compact ? 'mapp--compact' : ''} ${className}`}>
      {chrome === 'windows' && (
        <div className="mapp-caption">
          <span className="mapp-caption-title"><MurmurIcon size={16} />Murmur</span>
          <WindowControls os="windows" />
        </div>
      )}
      <aside className="mapp-side">
        {chrome === 'mac' && <WindowControls os="mac" />}
        {chrome === 'mac' && <div className="mapp-brand"><MurmurIcon size={22} /><b>Murmur</b><span>v0.4</span></div>}
        <nav aria-label="Murmur settings, example">
          {NAV.map((n) => (
            <button key={n.id} type="button" className={n.id === cur ? 'on' : ''} aria-current={n.id === cur ? 'page' : undefined} onClick={() => { setCur(n.id); setMoved(true); }}>
              {n.icon}{n.label}
            </button>
          ))}
        </nav>
        <div className="mapp-status"><i />Ready</div>
      </aside>
      {/* swaps on the click itself; a short CSS fade-in, no exit wait */}
      <div key={cur} className={`mapp-main ${moved ? 'mapp-main--enter' : ''}`}>
        {cur === page && children ? children : <DefaultPage id={cur} os={os} />}
      </div>
    </div>
  );
}

/* ---------- 3D keycap (CSS does the press) ---------- */
export function Key({ label, wide = 1, pressed }: { label: string; wide?: number; pressed: boolean }) {
  return (
    <kbd className={`key ${label.length > 1 ? 'key--word' : ''} ${pressed ? 'is-down' : ''}`} style={{ ['--kw' as string]: wide }}>
      <span className="key-top">{label}</span>
    </kbd>
  );
}

/* ---------- Inline keycaps for copy: <kbd><kbd>⌥</kbd><kbd>Space</kbd></kbd> ---------- */
export function Keys({ keys }: { keys: string[] }) {
  return (
    <kbd className="kbds" aria-label={keys.map((k) => KEY_NAME[k] ?? k).join(' + ')}>
      {keys.map((k) => <kbd key={k} className="kbd" aria-hidden>{k}</kbd>)}
    </kbd>
  );
}
