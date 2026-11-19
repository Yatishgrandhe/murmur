import { KEY_NAME } from '../site';
import type { CSSProperties, ReactNode } from 'react';
import { IconBook, IconClock, IconKey, IconSparkle, IconWave } from './Icons';
import { MurmurIcon } from './MurmurIcon';
import { Waveform } from './Waveform';

/* ---------- macOS window ---------- */
export function MacWindow({ title, children, className = '', dark = false, style }: { title?: ReactNode; children: ReactNode; className?: string; dark?: boolean; style?: CSSProperties }) {
  return (
    <div className={`win ${dark ? 'win--dark' : ''} ${className}`} style={style}>
      <div className="win-bar">
        <span className="lights" aria-hidden><i /><i /><i /></span>
        {title && <span className="win-title">{title}</span>}
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

/* ---------- Murmur's own settings window ---------- */
const NAV = [
  { id: 'ai', label: 'AI Rewrite', icon: <IconSparkle size={15} /> },
  { id: 'shortcut', label: 'Shortcut', icon: <IconKey size={15} /> },
  { id: 'voice', label: 'Voice', icon: <IconWave size={15} /> },
  { id: 'dictionary', label: 'Dictionary', icon: <IconBook size={15} /> },
  { id: 'history', label: 'History', icon: <IconClock size={15} /> },
];
export function MurmurApp({ page, children, dark = false, className = '', compact = false }: { page: string; children: ReactNode; dark?: boolean; className?: string; compact?: boolean }) {
  return (
    <div className={`mapp ${dark ? 'mapp--dark' : ''} ${compact ? 'mapp--compact' : ''} ${className}`}>
      <aside className="mapp-side">
        <span className="lights" aria-hidden><i /><i /><i /></span>
        <div className="mapp-brand"><MurmurIcon size={22} /><b>Murmur</b><span>v0.3</span></div>
        <nav>
          {NAV.map((n) => (
            <span key={n.id} className={n.id === page ? 'on' : ''}>{n.icon}{n.label}</span>
          ))}
        </nav>
        <div className="mapp-status"><i />Ready</div>
      </aside>
      <div className="mapp-main">{children}</div>
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
