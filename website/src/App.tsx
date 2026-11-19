import { useCallback, useEffect, useState } from 'react';
import { Hero } from './components/Hero';
import { Loader } from './components/Loader';
import { Nav } from './components/Nav';
import { Everywhere, Mess, Shortcut, Wake } from './components/SectionsA';
import { Finale, Install, Memory, Privacy, WindowsSetup, YourAi } from './components/SectionsB';
import { detectOS } from './site';

export default function App() {
  const [os, setOs] = useState(detectOS);
  const [loading, setLoading] = useState(true);
  const [showLoader, setShowLoader] = useState(true);
  const done = useCallback(() => setLoading(false), []);

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
        <YourAi />
        <Memory />
        <Privacy os={os} />
        <Install os={os} onOs={setOs} />
        <WindowsSetup />
        <Finale os={os} />
      </main>
    </>
  );
}
