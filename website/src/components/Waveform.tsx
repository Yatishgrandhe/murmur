import { useEffect, useRef } from 'react';

type Props = {
  /** 0..1 target loudness; eased internally */
  level: number;
  bars?: number;
  className?: string;
  /** follow the pointer horizontally to create a local swell */
  interactive?: boolean;
  /** solid bar colour; defaults to the logo's palette */
  color?: string;
};

/** Canvas waveform with a three-stop gradient. Cheap: one rAF loop, no React re-renders. */
export function Waveform({ level, bars = 64, className, interactive = false, color }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelRef = useRef(level);
  levelRef.current = level;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    let raf = 0;
    let eased = levelRef.current;
    let pointerX = -1;
    let pointerEase = -1;
    const heights = new Float32Array(bars);

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const r = canvas.getBoundingClientRect();
      canvas.width = Math.max(1, r.width * dpr);
      canvas.height = Math.max(1, r.height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);

    const onMove = (e: PointerEvent) => {
      const r = canvas.getBoundingClientRect();
      const inside = e.clientY > r.top - 200 && e.clientY < r.bottom + 200;
      pointerX = inside ? (e.clientX - r.left) / r.width : -1;
    };
    if (interactive) window.addEventListener('pointermove', onMove);

    const draw = (t: number) => {
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;
      ctx.clearRect(0, 0, w, h);
      eased += (levelRef.current - eased) * 0.08;
      pointerEase = pointerX < 0 ? pointerEase * 0.95 - 0.02 : pointerEase + (pointerX - pointerEase) * 0.12;

      const gap = w / bars;
      const bw = Math.max(2, gap * 0.42);
      const grad = ctx.createLinearGradient(0, 0, w, 0);
      grad.addColorStop(0, '#0071e3');
      grad.addColorStop(1, '#5ac8fa');
      ctx.fillStyle = color ?? grad;

      const time = reduce ? 0 : t / 1000;
      for (let i = 0; i < bars; i++) {
        const x = i / (bars - 1);
        const envelope = Math.sin(Math.PI * x) ** 1.4;
        const n =
          Math.sin(i * 0.55 + time * 3.1) * 0.5 +
          Math.sin(i * 0.23 - time * 2.3) * 0.35 +
          Math.sin(i * 1.7 + time * 5.2) * 0.15;
        let target = 0.06 + eased * envelope * (0.55 + 0.45 * Math.abs(n));
        if (interactive && pointerEase >= 0) {
          const d = Math.abs(x - pointerEase);
          target += Math.exp(-(d * d) / 0.006) * 0.55;
        }
        heights[i] += (Math.min(target, 1) - heights[i]) * 0.2;
        const bh = Math.max(bw, heights[i] * h);
        const cx = i * gap + gap / 2 - bw / 2;
        const y = (h - bh) / 2;
        ctx.globalAlpha = 0.35 + 0.65 * Math.min(1, heights[i] * 1.6);
        ctx.beginPath();
        ctx.roundRect(cx, y, bw, bh, bw / 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      window.removeEventListener('pointermove', onMove);
    };
  }, [bars, interactive, color]);

  return <canvas ref={canvasRef} className={className} aria-hidden />;
}
