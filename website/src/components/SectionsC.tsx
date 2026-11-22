import { AnimatePresence, motion } from 'framer-motion';
import { useState, type ReactNode } from 'react';
import { PASTE, SHORTCUT, type OS } from '../site';
import { useInstaller } from '../useInstaller';
import { AppleGlyph, IconCheck, IconCopy, WindowsGlyph } from './Icons';
import { Keys } from './Mock';
import { MurmurIcon } from './MurmurIcon';
import { Segmented } from './SectionsA';

type Step = { h: string; body: ReactNode; aside?: ReactNode };

function Steps({ steps }: { steps: Step[] }) {
  return (
    <ol className="steps" style={{ ['--n' as string]: steps.length }}>
      {steps.map((st, i) => (
        <li key={st.h}>
          <span className="step-n" aria-hidden>{i + 1}</span>
          <div>
            <h3>{st.h}</h3>
            <p>{st.body}</p>
          </div>
          {st.aside && <div className="step-aside" aria-hidden>{st.aside}</div>}
        </li>
      ))}
    </ol>
  );
}

/* ---------- Connect your AI (white) ---------- */
const VENDORS = {
  claude: { tool: 'Claude Code', plan: 'a Claude Pro or Max plan', maker: 'Anthropic', cmd: 'claude auth login', site: 'claude.ai' },
  codex: { tool: 'Codex', plan: 'a ChatGPT plan', maker: 'OpenAI', cmd: 'codex login', site: 'chatgpt.com' },
} as const;
type Vendor = keyof typeof VENDORS;

export function Connect({ os }: { os: OS }) {
  const [v, setV] = useState<Vendor>('claude');
  const x = VENDORS[v];
  const mac = os === 'mac';
  const steps: Step[] = [
    {
      h: `Install ${x.tool}`,
      body: <>For {x.plan}. Murmur’s <b>Install</b> button runs {x.maker}’s own installer.</>,
      aside: <span className="mini-row"><span>{x.tool}</span><span className="mini-btn is-pri">Install</span></span>,
    },
    {
      h: 'Sign in',
      body: <>Murmur opens a terminal running <code>{x.cmd}</code>, which opens {x.site} in your browser. You sign in there, with your own account. Murmur never sees your password or tokens.</>,
      aside: <span className={`mini-term ${mac ? '' : 'mini-term--win'}`}><span>{mac ? '%' : 'PS>'}</span> {x.cmd}</span>,
    },
    {
      h: 'Verify',
      body: <>Murmur sends one real test message through your account. It says <b>Connected</b> only once that message goes through.</>,
      aside: (
        <span className="states">
          <span className="state"><i className="dot dot--off" />Not connected</span>
          <span className="state"><i className="dot dot--warn" />Signed in, not verified</span>
          <span className="state"><i className="dot dot--ok" />Connected, after the test</span>
        </span>
      ),
    },
  ];
  return (
    <section className="sec sec--white connect" id="connect">
      <div className="wrap steps-inner">
        <div className="steps-head">
          <h2>Connect your AI</h2>
          <p className="body">Three steps, once. After that, every rewrite runs on the plan you already pay for.</p>
          <Segmented label="AI plan" value={v} onChange={setV} options={[{ id: 'claude', label: 'Claude' }, { id: 'codex', label: 'ChatGPT' }]} />
        </div>
        <AnimatePresence mode="wait" initial={false}>
          <motion.div key={v} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: 0.2 }}>
            <Steps steps={steps} />
          </motion.div>
        </AnimatePresence>
      </div>
    </section>
  );
}

/* ---------- Permissions (light) ---------- */
export function Permissions({ os }: { os: OS }) {
  const mac = os === 'mac';
  const rows: { name: string; why: ReactNode; tag?: string }[] = mac
    ? [
        { name: 'Microphone', why: 'So it can hear you while you hold the shortcut.' },
        { name: 'Speech Recognition', why: 'So macOS can turn what you said into text.' },
        { name: 'Accessibility', tag: 'Optional', why: <>So it can paste for you. Without it, the text waits on the clipboard and you press <Keys keys={PASTE.mac} /> yourself.</> },
      ]
    : [
        { name: 'Microphone for desktop apps', why: 'So it can hear you while you hold the shortcut.' },
        { name: 'Online speech recognition', why: 'So Windows can turn what you said into text. It sends the audio you dictate to Microsoft.' },
      ];
  return (
    <section className="sec sec--light perms" id="permissions">
      <div className="wrap perms-inner">
        <div className="perms-copy">
          <h2>It asks only when you click</h2>
          <p className="body">Each permission has its own button in setup. Nothing pops up until you press it.</p>
          <p className="perms-never">Murmur never asks for Screen Recording or Full Disk Access.</p>
        </div>
        <ul className="perms-list">
          {rows.map((r) => (
            <li key={r.name}>
              <div>
                <h3>{r.name}{r.tag && <span className="tag">{r.tag}</span>}</h3>
                <p>{r.why}</p>
              </div>
              <span className="mini-btn" aria-hidden>Allow</span>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}

/* ---------- Install (white): steps per OS, build-from-source tucked away ---------- */
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

const fileLink = (url: string) => (
  <a className="link" href={url}>{url.split('/').pop()!.split('_').map((part, i) => <span key={i}>{i > 0 && <>_<wbr /></>}{part}</span>)}</a>
);

export function Install({ os, onOs }: { os: OS; onOs: (o: OS) => void }) {
  const { copied, copy } = useCopy();
  const exe = useInstaller();
  const mac = os === 'mac';

  const macSteps: Step[] = [
    {
      h: 'Download',
      body: <>{exe.dmg ? fileLink(exe.dmg.url) : <a className="link" href="https://github.com/Yatishgrandhe/murmur/releases/latest">the .dmg</a>}, about {exe.dmg?.size ?? '4 MB'}, from the latest release.</>,
    },
    {
      h: 'Drag it into Applications',
      body: <>Open Murmur from Applications, not from the disk image. The first time, right-click it and choose <b>Open</b>. It isn’t notarized yet.</>,
      aside: <span className="mini-row"><MurmurIcon size={22} /><span>→</span><span className="mini-folder">Applications</span></span>,
    },
    {
      h: 'Allow and connect',
      body: <>Setup walks you through permissions and connecting your AI, one button at a time.</>,
    },
    {
      h: 'Talk',
      body: <>Hold <Keys keys={SHORTCUT.mac} />, speak, let go.</>,
    },
  ];
  const winSteps: Step[] = [
    {
      h: 'Download and run',
      body: <>{fileLink(exe.url)}, about {exe.size}. It installs for your user only, so no admin is needed. If SmartScreen appears, click <b>More info</b>, then <b>Run anyway</b>.</>,
      aside: <span className="mini-row"><span className="mini-btn">More info</span><span className="mini-btn is-pri">Run anyway</span></span>,
    },
    {
      h: 'Allow and connect',
      body: <>Setup asks for the microphone and Online speech recognition, then connects your AI, one button at a time.</>,
    },
    {
      h: 'Find it in the tray',
      body: <>Murmur lives in the system tray by the clock. Left-click its icon to open settings.</>,
      aside: <span className="win-tray"><MurmurIcon size={18} /><span>9:41 AM</span></span>,
    },
    {
      h: 'Talk',
      body: <>Hold <Keys keys={SHORTCUT.windows} />, speak, let go.</>,
    },
  ];

  return (
    <section className="sec sec--white install" id="install">
      <span id="windows" className="anchor" aria-hidden />
      <div className="wrap steps-inner">
        <div className="steps-head">
          <h2>Install it</h2>
          <p className="body">The app is under 10 MB, and that’s everything.</p>
          <Segmented label="Operating system" value={os} onChange={onOs} options={[{ id: 'mac', label: <><AppleGlyph size={14} /> macOS</> }, { id: 'windows', label: <><WindowsGlyph size={14} /> Windows</> }]} />
        </div>
        <AnimatePresence mode="wait" initial={false}>
          <motion.div key={os} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: 0.2 }}>
            <Steps steps={mac ? macSteps : winSteps} />
            <dl className="install-facts">
              <div><dt>Nothing else</dt><dd>No voice models, runtimes or background services.{!mac && ' It uses the WebView2 that comes with Windows 11 and current Windows 10.'}</dd></div>
              <div><dt>Reinstalling?</dt><dd>Setup skips the steps you’ve already done.</dd></div>
              <div><dt>Tidy up</dt><dd>Afterwards, Murmur offers to move the installer in Downloads to the {mac ? 'Trash' : 'Recycle Bin'}. Only if you click.</dd></div>
            </dl>
            <details className="build">
              <summary>Build it from source instead</summary>
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
                  <p className="term-out">Needs {mac ? 'Rust, Node.js and the Xcode command-line tools' : 'Rust, Node.js, the Visual Studio C++ build tools and WebView2'}.</p>
                </div>
              </div>
            </details>
          </motion.div>
        </AnimatePresence>
      </div>
    </section>
  );
}
