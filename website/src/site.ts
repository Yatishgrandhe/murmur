export const REPO = 'https://github.com/Yatishgrandhe/murmur';
export const RELEASES = `${REPO}/releases/latest`;
export const WIN_EXE = `${REPO}/releases/download/v0.1.0/Murmur_0.1.0_x64-setup.exe`;

export type OS = 'mac' | 'windows';
type NavigatorUA = Navigator & { userAgentData?: { platform?: string } };

export function detectOS(): OS {
  if (typeof navigator === 'undefined') return 'mac';
  const forced = new URLSearchParams(location.search).get('os');
  if (forced === 'mac' || forced === 'windows') return forced; // ?os=windows to preview the other OS
  const nav = navigator as NavigatorUA;
  const s = `${nav.userAgentData?.platform ?? ''} ${nav.userAgent}`.toLowerCase();
  return /windows|win32|win64/.test(s) ? 'windows' : 'mac';
}

/** Each visitor sees only their own OS's shortcut. Windows uses Ctrl+Shift+Space because Alt+Space opens the window menu there. */
export const SHORTCUT: Record<OS, string[]> = { mac: ['⌥', 'Space'], windows: ['Ctrl', 'Shift', 'Space'] };
export const shortcutText = (os: OS) => (os === 'mac' ? '⌥ Space' : 'Ctrl + Shift + Space');


/* ---------- Latest Windows installer, looked up from GitHub Releases ---------- */
export type Installer = { url: string; size: string; tag: string };
export const FALLBACK_INSTALLER: Installer = { url: WIN_EXE, size: '3.6 MB', tag: 'v0.1.0' };

let pending: Promise<Installer> | null = null;
/** One request per page load; falls back to the pinned v0.1.0 installer if GitHub can't be reached. */
export function latestInstaller(): Promise<Installer> {
  pending ??= fetch('https://api.github.com/repos/Yatishgrandhe/murmur/releases/latest', { headers: { Accept: 'application/vnd.github+json' } })
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((rel: { tag_name: string; assets: { name: string; size: number; browser_download_url: string }[] }) => {
      const exe = rel.assets.find((a) => /x64-setup\.exe$/i.test(a.name));
      if (!exe) return FALLBACK_INSTALLER;
      return { url: exe.browser_download_url, size: `${(exe.size / 1048576).toFixed(1)} MB`, tag: rel.tag_name };
    })
    .catch(() => FALLBACK_INSTALLER);
  return pending;
}
