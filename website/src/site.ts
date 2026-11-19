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
  const platform = (nav.userAgentData?.platform || nav.platform || nav.userAgent).toLowerCase();
  return /^win|windows/.test(platform) ? 'windows' : 'mac';
}

export const OS_NAME: Record<OS, string> = { mac: 'Mac', windows: 'Windows' };

/** Keycaps per OS. Windows keys are words, never ⌘/⌥ glyphs. Alt+Space opens the window menu on Windows, hence Ctrl+Shift+Space. */
export const SHORTCUT: Record<OS, string[]> = { mac: ['⌥', 'Space'], windows: ['Ctrl', 'Shift', 'Space'] };
export const PASTE: Record<OS, string[]> = { mac: ['⌘', 'V'], windows: ['Ctrl', 'V'] };
export const KEY_NAME: Record<string, string> = { '⌥': 'Option', '⌘': 'Command' };
export const shortcutText = (os: OS) => (os === 'mac' ? '⌥ Space' : 'Ctrl + Shift + Space');


/* ---------- Latest installers (.exe and .dmg), looked up from GitHub Releases ---------- */
export type MacDownload = { url: string; size: string };
export type Installer = { url: string; size: string; tag: string; dmg: MacDownload | null };
export const FALLBACK_INSTALLER: Installer = { url: WIN_EXE, size: '3.6 MB', tag: 'v0.1.0', dmg: null };
const mb = (bytes: number) => `${(bytes / 1048576).toFixed(1)} MB`;

let pending: Promise<Installer> | null = null;
/** One request per page load; falls back to the pinned v0.1.0 installer (and no Mac download) if GitHub can't be reached. */
export function latestInstaller(): Promise<Installer> {
  pending ??= fetch('https://api.github.com/repos/Yatishgrandhe/murmur/releases/latest', { headers: { Accept: 'application/vnd.github+json' } })
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((rel: { tag_name: string; assets: { name: string; size: number; browser_download_url: string }[] }) => {
      const exe = rel.assets.find((a) => /x64-setup\.exe$/i.test(a.name));
      const dmg = rel.assets.find((a) => /aarch64\.dmg$/i.test(a.name)) ?? rel.assets.find((a) => /\.dmg$/i.test(a.name));
      const mac = dmg ? { url: dmg.browser_download_url, size: mb(dmg.size) } : null;
      if (!exe) return { ...FALLBACK_INSTALLER, dmg: mac };
      return { url: exe.browser_download_url, size: mb(exe.size), tag: rel.tag_name, dmg: mac };
    })
    .catch(() => FALLBACK_INSTALLER);
  return pending;
}
