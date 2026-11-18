import { motion } from 'framer-motion';
import { useEffect } from 'react';
import { MurmurIcon } from './MurmurIcon';

/** Short splash in the spirit of the app's own opening: icon, moving bars, name. */
export function Loader({ onDone, leaving }: { onDone: () => void; leaving: boolean }) {
  useEffect(() => {
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const t = setTimeout(onDone, reduce ? 300 : 1500);
    return () => clearTimeout(t);
  }, [onDone]);

  return (
    <div className={`loader ${leaving ? 'is-leaving' : ''}`} role="status" aria-label="Loading Murmur">
      <motion.div
        className="loader-mark"
        initial={{ scale: 0.86, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ type: 'spring', stiffness: 220, damping: 20 }}
      >
        <MurmurIcon size={112} />
      </motion.div>
      <div className="loader-bars" aria-hidden>
        {Array.from({ length: 7 }).map((_, i) => (
          <span key={i} style={{ animationDelay: `${i * 0.08}s` }} />
        ))}
      </div>
      <motion.div
        className="loader-word"
        initial={{ opacity: 0, y: 6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.2, duration: 0.4, ease: 'easeOut' }}
      >
        Murmur
      </motion.div>
    </div>
  );
}
