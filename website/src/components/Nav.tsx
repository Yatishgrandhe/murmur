import { type OS } from '../site';
import { MurmurIcon } from './MurmurIcon';
import { useInstaller } from '../useInstaller';

export function Nav({ os }: { os: OS }) {
  const exe = useInstaller();
  return (
    <header className="nav">
      <div className="nav-inner">
        <a href="#top" className="nav-brand" aria-label="Murmur, back to top">
          <MurmurIcon size={22} />
          <span>Murmur</span>
        </a>
        <nav className="nav-links" aria-label="Sections">
          <a href="#styles">Styles</a>
          <a href="#how">Shortcut</a>
          <a href="#ai">AI</a>
          <a href="#privacy">Privacy</a>
          <a href="#install">Install</a>
        </nav>
        <a className="btn btn--sm btn--primary" href={os === 'windows' ? exe.url : exe.dmg?.url ?? '#install'}>Download</a>
      </div>
    </header>
  );
}
