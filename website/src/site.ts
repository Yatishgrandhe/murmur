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

