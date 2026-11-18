import { useInView, useReducedMotion } from 'framer-motion';
import { useEffect, useState, type RefObject } from 'react';

/** True while the element is on screen and the visitor hasn't asked for reduced motion. */
export function useLive(ref: RefObject<Element>, amount = 0.35) {
  const inView = useInView(ref, { amount });
  const reduce = useReducedMotion();
  return inView && !reduce;
}

/** Increments every `ms` while active. */
export function useTick(ms: number, active: boolean) {
  const [t, setT] = useState(0);
  useEffect(() => {
    if (!active) return;
    const id = setInterval(() => setT((x) => x + 1), ms);
    return () => clearInterval(id);
  }, [ms, active]);
  return t;
}

/** Types `text` out when `run` is true (restarts when `nonce` changes); shows it whole otherwise. */
export function useTyped(text: string, run: boolean, speed = 28, nonce = 0) {
  const [n, setN] = useState(text.length);
  useEffect(() => {
    if (!run) {
      setN(text.length);
      return;
    }
    setN(0);
    let i = 0;
    const id = setInterval(() => {
      i += 1;
      setN(i);
      if (i >= text.length) clearInterval(id);
    }, speed);
    return () => clearInterval(id);
  }, [text, run, speed, nonce]);
  return { shown: text.slice(0, n), done: n >= text.length };
}
