import { type OS } from '../site';
import { Segmented } from './SectionsA';
import { MurmurIcon } from './MurmurIcon';
import { useInstaller } from '../useInstaller';

export function Nav({ os, onOs }: { os: OS; onOs: (o: OS) => void }) {
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
          <a href="#windows">Windows</a>
        </nav>
        <div className="nav-end">
          <Segmented label="Show keys and steps for" tone="dark" size="sm" kind="radio" value={os} onChange={onOs} options={[{ id: 'mac', label: 'Mac' }, { id: 'windows', label: 'Windows' }]} />
          <a className="btn btn--sm btn--primary" href={os === 'windows' ? exe.url : exe.dmg?.url ?? '#install'}>Download</a>
        </div>
      </div>
    </header>
  );
}
