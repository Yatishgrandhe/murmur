import { AnimatePresence, motion } from 'framer-motion';
import { useEffect, useId, useRef, useState } from 'react';
import { useLive, useTick, useTyped } from '../hooks';
import { PASTE, SHORTCUT, type OS } from '../site';
import { Key, Keys, MacWindow, Pill } from './Mock';
import { Waveform } from './Waveform';

/* ---------- Segmented control (real UI, used in several sections) ---------- */
export function Segmented<T extends string>({ value, options, onChange, label, tone = 'light', size = 'md', kind = 'tabs' }: { value: T; options: { id: T; label: React.ReactNode }[]; onChange: (v: T) => void; label: string; tone?: 'light' | 'dark'; size?: 'sm' | 'md'; kind?: 'tabs' | 'radio' }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const gid = useId();
  const onKey = (e: React.KeyboardEvent, i: number) => {
    if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
    e.preventDefault();
    const n = (i + (e.key === 'ArrowRight' ? 1 : -1) + options.length) % options.length;
    onChange(options[n].id);
    refs.current[n]?.focus();
  };
  return (
    <div className={`seg seg--${tone} seg--${size}`} role={kind === 'radio' ? 'radiogroup' : 'tablist'} aria-label={label}>
      {options.map((o, i) => {
        const on = o.id === value;
        return (
          <button key={o.id} ref={(el) => { refs.current[i] = el; }} role={kind === 'radio' ? 'radio' : 'tab'} type="button" aria-selected={kind === 'tabs' ? on : undefined} aria-checked={kind === 'radio' ? on : undefined} tabIndex={on ? 0 : -1}
            className={`seg-btn ${on ? 'is-on' : ''}`} onClick={() => onChange(o.id)} onKeyDown={(e) => onKey(e, i)}>
            {on && <motion.span layoutId={`seg-${gid}`} className="seg-thumb" transition={{ duration: 0.2, ease: 'easeOut' }} />}
            <span className="seg-label">{o.label}</span>
          </button>
        );
      })}
    </div>
  );
}

/* ---------- 2. The mess → the message (light) ---------- */
const WORDS: [string, boolean][] = [
  ['um', true], ['so', true], ['I', true], ['was', true], ['thinking', true], ['we', false], ['should', false], ['meet', false], ['on', false],
  ['tuesday', true], ['no', true], ['wait', true], ['wednesday', false], ['at', false], ['like', true], ['three', false], ['to', false],
  ['talk', false], ['about', false], ['the', false], ['launch', false],
];
const TONES = {
  clean: 'We should meet on Wednesday at 3 to talk about the launch.',
  polished: 'Let’s meet Wednesday at 3 PM to discuss the launch.',
  casual: 'let’s meet wednesday at 3 to talk about the launch',
  email: 'Hi,\n\nCould we meet on Wednesday at 3 to talk about the launch?',
  prompt: 'Goal: set up a meeting about the launch.\nWhen: Wednesday, 3 PM.\nOutput: a short calendar invite.',
  custom: '• Meet Wednesday, 3 PM\n• Topic: the launch',
} as const;
type Tone = keyof typeof TONES;
const TONE_LABEL: Record<Tone, string> = { clean: 'Clean', polished: 'Polished', casual: 'Casual', email: 'Email', prompt: '✦ Prompt', custom: 'Custom' };
const TONE_NOTE: Partial<Record<Tone, string>> = { prompt: 'Prompt turns what you say into a structured prompt for an AI.', custom: 'Custom follows your own instructions. Here: “bullet points”.' };

export function Mess() {
  const ref = useRef<HTMLElement>(null);
  const live = useLive(ref);
  const tick = useTick(3200, live);
  const struck = !live || tick % 2 === 1;
  const [tone, setTone] = useState<Tone>('clean');
  const out = useTyped(TONES[tone], live, 16, tick === 0 ? 0 : 1);

  return (
    <section className="sec sec--light mess" id="styles" ref={ref}>
      <div className="wrap mess-inner">
        <h2>You ramble. It doesn’t</h2>
        <div className="mess-split">
          <div className="mess-col">
            <span className="mess-label">What you said</span>
            <p className={`mess-raw ${struck ? 'is-struck' : ''}`}>
              {WORDS.map(([w, filler], i) => (
                <span key={i} className={filler ? 'filler' : ''} style={{ transitionDelay: `${i * 25}ms` }}>{w} </span>
              ))}
            </p>
          </div>
          <div className="mess-arrow" aria-hidden>→</div>
          <div className="mess-col">
            <span className="mess-label">What lands</span>
            <p className="mess-out" aria-live="polite">{out.shown}<span className="caret caret--blue" /></p>
          </div>
        </div>
        <div className="mess-tones">
          <Segmented label="Writing style" value={tone} onChange={setTone} options={(Object.keys(TONES) as Tone[]).map((k) => ({ id: k, label: TONE_LABEL[k] }))} />
          <span className="mess-note">{TONE_NOTE[tone] ?? 'Pick a style under AI Rewrite. These are examples, not live output.'}</span>
        </div>
      </div>
    </section>
  );
}

/* ---------- 3. One shortcut (black) ---------- */
export function Shortcut({ os }: { os: OS }) {
  const ref = useRef<HTMLElement>(null);
  const live = useLive(ref);
  // Hovering pauses the demo loop; pointer-down (mouse, pen or touch) presses the keys and holding keeps them down.
  const [hover, setHover] = useState(false);
  const [held, setHeld] = useState(false);
  const tick = useTick(1500, live && !hover && !held);
  const pressed = held || (live && !hover && tick % 2 === 0);
  const keys = SHORTCUT[os];
  const widths: Record<string, number> = os === 'mac' ? { Space: 3.2 } : { Ctrl: 1.25, Shift: 1.6, Space: 2.4 };
  return (
    <section className="sec sec--black shortcut" id="how" ref={ref}>
      <div className="wrap shortcut-inner">
        <h2>One shortcut</h2>
        <div className="shortcut-stage">
          <div className="shortcut-pill">
            <AnimatePresence>
              {pressed && (
                <motion.div initial={{ opacity: 0, y: 10, scale: 0.9 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: 6 }} transition={{ duration: 0.22 }}>
                  <Pill state="listen" text={held ? "Listening. Let go to paste" : "Listening"} level={0.8} />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
          <div className={`keys ${held ? 'is-held' : ''}`}
            onPointerEnter={(e) => e.pointerType === 'mouse' && setHover(true)}
            onPointerLeave={() => setHover(false)}
            onPointerDown={(e) => {
              setHeld(true);
              try { e.currentTarget.setPointerCapture(e.pointerId); } catch { /* pointer already gone */ }
            }}
            onPointerUp={() => setHeld(false)}
            onPointerCancel={() => setHeld(false)}
            onLostPointerCapture={() => setHeld(false)}
            onContextMenu={(e) => e.preventDefault()} aria-label={`Shortcut: ${keys.join(' + ')}`} role="img">
            {keys.map((k) => <Key key={k} label={k} wide={widths[k] ?? 1} pressed={pressed} />)}
          </div>
        </div>
        <ul className="shortcut-lines">
          <li><Keys keys={keys} /><span><b>Hold</b> to talk. Let go and it’s pasted where your cursor is.</span></li>
          <li><Keys keys={keys} /><span><b>Tap once</b> for hands-free. It stops when you pause, or tap again to finish.</span></li>
          <li><Keys keys={['Esc']} /><span><b>Cancel.</b> Nothing gets typed.</span></li>
          <li><Keys keys={PASTE[os]} /><span><b>Paste it again.</b> The last dictation stays on the clipboard, even if no text field had focus.</span></li>
        </ul>
        <p className="shortcut-note">Want different keys? Change them in Murmur › Shortcut.</p>
      </div>
    </section>
  );
}

/* ---------- 4. Hey Murmur (cobalt) ---------- */
export function Wake({ os }: { os: OS }) {
  const ref = useRef<HTMLElement>(null);
  const live = useLive(ref);
  const tick = useTick(2600, live);
  const level = live ? (tick % 2 === 0 ? 0.95 : 0.35) : 0.6;
  return (
    <section className="sec sec--blue wake" ref={ref}>
      <div className="wrap wake-inner">
        <p className="wake-kicker">Or don’t touch the keyboard at all</p>
        <blockquote className="wake-quote">“Hey Murmur, tell John I’m running late.”</blockquote>
        <Waveform level={level} bars={72} color="#ffffff" className="wake-meter" />
        <p className="wake-line">
          Say “Hey Murmur”, pause, then talk. Or say it all in one breath. It’s off until you turn it on in Murmur › Shortcut.{' '}
          {os === 'windows'
            ? 'Windows’ offline phrase recognizer listens for it, so background audio isn’t uploaded.'
            : 'Your Mac listens for it with on-device recognition where your Mac supports it.'}
        </p>
      </div>
    </section>
  );
}

/* ---------- 5. Works everywhere (white) ---------- */
const APPS = [
  { id: 'code', title: 'llm.rs — Editor', text: '// fall back to the raw transcript if the AI call fails', kind: 'code' },
  { id: 'notes', title: 'Notes', text: 'Buy oat milk, eggs, and the good coffee.', kind: 'notes' },
  { id: 'mail', title: 'New Message', text: 'Hi Sam, thanks for the notes. I’ll send the revised deck by Friday.', kind: 'mail' },
  { id: 'chat', title: '#launch', text: 'Can we push the demo to 3? Still fixing the pill animation.', kind: 'chat' },
] as const;
type App = (typeof APPS)[number];

function AppWindow({ app, front, nonce, live }: { app: App; front: boolean; nonce: number; live: boolean }) {
  const { shown } = useTyped(app.text, live && front, 22, nonce);
  const text = front ? shown : '';
  const caret = front ? <span className={`caret ${app.kind === 'code' ? '' : 'caret--blue'}`} /> : null;
  return (
    <MacWindow className={`cas cas--${app.kind} ${front ? 'is-front' : ''}`} title={app.title} dark={app.kind === 'code'}>
      {app.kind === 'mail' && (
        <div className="cas-mail">
          <div className="cas-field">To: <b>Sam Rivera</b></div>
          <div className="cas-field">Subject: <b>Deck notes</b></div>
          <p>{text}{caret}</p>
        </div>
      )}
      {app.kind === 'code' && (
        <pre className="cas-code">
          <span className="ln">41</span><span className="kw">fn</span> rewrite(text: &amp;str) -&gt; String {'{'}{'\n'}
          <span className="ln">42</span>    <span className="cm">{text}</span>{caret}{'\n'}
          <span className="ln">43</span>    provider.finish(text).unwrap_or(text.into()){'\n'}
          <span className="ln">44</span>{'}'}
        </pre>
      )}
      {app.kind === 'notes' && <div className="cas-notes"><b>Groceries</b><p>{text}{caret}</p></div>}
      {app.kind === 'chat' && (
        <div className="cas-chat">
          <p className="cas-msg"><b>Priya</b> Demo still at 2?</p>
          <div className="cas-compose">{text}{caret}{!front && <span className="ph">Message #launch</span>}</div>
        </div>
      )}
    </MacWindow>
  );
}

export function Everywhere() {
  const ref = useRef<HTMLElement>(null);
  const live = useLive(ref, 0.25);
  // back-to-front order; every 3.4 s the back window comes forward, and a click brings any window forward now
  const [order, setOrder] = useState<App[]>(() => [...APPS]);
  const [nonce, setNonce] = useState(0);
  const bringForward = (id: App['id']) => {
    setOrder((o) => [...o.filter((a) => a.id !== id), o.find((a) => a.id === id)!]);
    setNonce((n) => n + 1);
  };
  useEffect(() => {
    if (!live) return;
    const t = window.setTimeout(() => bringForward(order[0].id), 3400);
    return () => clearTimeout(t);
  }, [live, nonce, order]);
  return (
    <section className="sec sec--white everywhere" ref={ref}>
      <div className="wrap everywhere-inner">
        <div className="everywhere-copy">
          <h2>Wherever your cursor is</h2>
          <p className="body">Chat, mail, notes, your code editor. If you can type in it, Murmur can paste into it.</p>
        </div>
        <div className="fan">
          {order.map((a, k) => {
            const front = k === order.length - 1;
            return (
              <motion.button key={a.id} type="button" className={`fan-slot ${front ? 'is-front' : ''}`} style={{ zIndex: k + 1 }} aria-label={front ? `${a.title}, in front` : `Bring ${a.title} to the front`} aria-pressed={front}
                onClick={() => !front && bringForward(a.id)} animate={{ x: k * 40, y: k * 40 }} transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}>
                <AppWindow app={a} front={front} nonce={nonce} live={live} />
              </motion.button>
            );
          })}
        </div>
      </div>
    </section>
  );
}
