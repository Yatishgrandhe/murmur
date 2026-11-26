import { useCallback, useEffect, useState } from 'react';
import { Hero } from './components/Hero';
import { Loader } from './components/Loader';
import { Nav } from './components/Nav';
import { Everywhere, Mess, Shortcut, Wake } from './components/SectionsA';
import { Finale, Memory, Privacy, YourAi } from './components/SectionsB';
import { Connect, Install, Permissions } from './components/SectionsC';
import { detectOS } from './site';

export default function App() {
  const [os, setOs] = useState(detectOS);
  const [loading, setLoading] = useState(true);
  const [showLoader, setShowLoader] = useState(true);
  const done = useCallback(() => setLoading(false), []);

  // Old links to #windows land on the install steps with Windows selected.
  useEffect(() => {
    const sync = () => location.hash === '#windows' && setOs('windows');
    sync();
    addEventListener('hashchange', sync);
    return () => removeEventListener('hashchange', sync);
  }, []);

  // CSS fade + timer unmount: works even when rAF is paused in a background tab.
  useEffect(() => {
    if (loading) return;
    const t = setTimeout(() => setShowLoader(false), 500);
    return () => clearTimeout(t);
  }, [loading]);

  return (
    <>
      {showLoader && <Loader leaving={!loading} onDone={done} />}
      <Nav os={os} onOs={setOs} />
      <main>
        <Hero os={os} ready={!loading} />
        <Mess />
        <Shortcut os={os} />
        <Wake os={os} />
        <Everywhere />
        <YourAi os={os} />
        <Connect os={os} />
        <Memory os={os} />
        <Privacy os={os} />
        <Permissions os={os} />
        <Install os={os} onOs={setOs} />
        <Finale os={os} />
      </main>
    </>
  );
}
