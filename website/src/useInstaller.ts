import { useEffect, useState } from 'react';
import { FALLBACK_INSTALLER, latestInstaller, type Installer } from './site';

export function useInstaller(): Installer {
  const [inst, setInst] = useState<Installer>(FALLBACK_INSTALLER);
  useEffect(() => {
    let alive = true;
    latestInstaller().then((i) => alive && setInst(i));
    return () => {
      alive = false;
    };
  }, []);
  return inst;
}
