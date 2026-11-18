type Props = { size?: number; className?: string };

/** The real Murmur app icon, from the murmur repo (ui/logo.png). */
export function MurmurIcon({ size = 40, className }: Props) {
  return <img src="./murmur-logo.png" width={size} height={size} alt="" className={className} draggable={false} />;
}
