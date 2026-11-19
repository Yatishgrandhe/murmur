import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { useEffect, useRef, useState } from 'react';
import { useLive } from '../hooks';
import { SHORTCUT, type OS } from '../site';
import { useInstaller } from '../useInstaller';
import { AppleGlyph, WindowsGlyph } from './Icons';
import { Keys, MacWindow, Pill, type PillState } from './Mock';

const RAW = 'um so tell john I’m like running ten min late';
const CLEAN = 'Running about 10 minutes late!';

type Step = 'idle' | 'listen' | 'polish' | 'type' | 'sent';

/** One loop: pill appears → messy speech → polishing → clean text types into the box → sent. */
function useHeroLoop(active: boolean) {
  const [step, setStep] = useState<Step>('sent');
  const [raw, setRaw] = useState(0);
  const [clean, setClean] = useState(CLEAN.length);

  useEffect(() => {
    if (!active) {
      setStep('sent');
      setRaw(RAW.length);
      setClean(CLEAN.length);
      return;
    }
    const timers: number[] = [];
    const at = (ms: number, fn: () => void) => timers.push(window.setTimeout(fn, ms));
    const run = () => {
      let t = 0;
      at(t, () => { setStep('idle'); setRaw(0); setClean(0); });
      t += 700;
      at(t, () => setStep('listen'));
      for (let i = 1; i <= RAW.length; i++) at(t + 120 + i * 38, () => setRaw(i));
      t += 120 + RAW.length * 38 + 450;
      at(t, () => setStep('polish'));
      t += 900;
      at(t, () => setStep('type'));
      for (let i = 1; i <= CLEAN.length; i++) at(t + i * 26, () => setClean(i));
      t += CLEAN.length * 26 + 500;
      at(t, () => setStep('sent'));
      t += 3200;
      at(t, run);
    };
    run();
    return () => timers.forEach(clearTimeout);
  }, [active]);

  return { step, raw: RAW.slice(0, raw), clean: CLEAN.slice(0, clean) };
}

export function DownloadButtons({ os, tone = 'dark', size = 'lg' }: { os: OS; tone?: 'dark' | 'light'; size?: 'lg' | 'md' }) {
  const exe = useInstaller();
  const cls = (mine: boolean) => `btn btn--${size} ${mine ? 'btn--primary' : `btn--secondary btn--on-${tone}`}`;
  const mac = (
    <a key="mac" href={exe.dmg?.url ?? '#install'} className={cls(os === 'mac')}>
      <AppleGlyph size={16} /> Get it for Mac
    </a>
  );
  const win = (
    <a key="win" href={exe.url} className={cls(os === 'windows')}>
      <WindowsGlyph size={16} /> Get it for Windows
    </a>
  );
  return <div className="ctas">{os === 'windows' ? [win, mac] : [mac, win]}</div>;
}

export function Hero({ os, ready }: { os: OS; ready: boolean }) {
  const ref = useRef<HTMLElement>(null);
  const live = useLive(ref, 0.3) && ready;
  const reduce = useReducedMotion();
  const { step, raw, clean } = useHeroLoop(live);

  const pillState: PillState | null = step === 'listen' ? 'listen' : step === 'polish' ? 'polish' : step === 'type' || step === 'sent' ? 'done' : null;
  const enter = (d: number) => (reduce ? {} : { initial: { opacity: 0, y: 16 }, animate: ready ? { opacity: 1, y: 0 } : undefined, transition: { delay: d, duration: 0.6, ease: [0.22, 1, 0.36, 1] } });

  return (
    <section className="sec hero" id="top" ref={ref}>
      <div className="wallpaper wallpaper--hero" aria-hidden />
      <div className="hero-inner">
        <div className="hero-copy">
          <motion.h1 {...enter(0)}>Talk.<br />It types.</motion.h1>
          <motion.p className="hero-sub" {...enter(0.08)}>
            Hold <Keys keys={SHORTCUT[os]} />, ramble, let go. Murmur cleans it up and pastes it where your cursor is.
          </motion.p>
          <motion.div {...enter(0.16)}><DownloadButtons os={os} /></motion.div>
          <motion.p className="hero-fine" {...enter(0.22)}>Free · Open source · About 10 MB · Uses the Claude or ChatGPT plan you already have</motion.p>
        </div>

        <motion.div className="hero-stage" {...enter(0.12)}>
          <MacWindow className="msgs" title="John">
            <div className="msgs-grid">
              <aside className="msgs-list" aria-hidden>
                {[['John', 'Table’s ready whenever 🍜', true], ['Priya', 'Sent the deck', false], ['Mom', 'Call me back', false], ['Launch crew', 'Ship it Wednesday?', false]].map(([n, m, on]) => (
                  <div key={n as string} className={`msgs-item ${on ? 'on' : ''}`}>
                    <span className="avatar">{(n as string)[0]}</span>
                    <span><b>{n}</b><small>{m}</small></span>
                  </div>
                ))}
              </aside>
              <div className="msgs-chat">
                <div className="bubbles">
                  <p className="b them">Dinner at 7 still good?</p>
                  <p className="b me">Yep, leaving soon</p>
                  <p className="b them">Table’s ready whenever 🍜</p>
                  <AnimatePresence>
                    {step === 'sent' && (
                      <motion.p key="new" className="b me" initial={{ opacity: 0, y: 16, scale: 0.9 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0 }} transition={{ type: 'spring', stiffness: 380, damping: 26 }}>
                        {CLEAN}
                      </motion.p>
                    )}
                  </AnimatePresence>
                </div>
                <div className="compose">
                  <span className={step === 'type' ? '' : 'ph'}>{step === 'type' ? clean : 'iMessage'}</span>
                  {step === 'type' && <span className="caret" />}
                </div>
              </div>
            </div>
          </MacWindow>
          <div className="hero-pill">
            <AnimatePresence mode="wait">
              {pillState && (
                <motion.div key={pillState} initial={{ opacity: 0, y: 10, scale: 0.92 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: 6, scale: 0.96 }} transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}>
                  <Pill state={pillState} text={pillState === 'listen' ? raw : CLEAN} />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </motion.div>
      </div>

    </section>
  );
}
