import {
  BookOpen, Briefcase, Camera, Clock, Compass, Cpu, Folder, Glasses, Globe, Heart, House, Landmark, Layers, LayoutGrid,
  Leaf, Lightbulb, Monitor, Music, Palette, PenTool, Sparkles, Star, Target, Zap, type LucideIcon,
} from 'lucide-react';

/** Board icons by stored name (the same names Milaclone uses). */
export const BOARD_ICONS: Record<string, LucideIcon> = {
  'layout-grid': LayoutGrid,
  'book-open': BookOpen,
  monitor: Monitor,
  clock: Clock,
  heart: Heart,
  house: House,
  lightbulb: Lightbulb,
  palette: Palette,
  briefcase: Briefcase,
  sparkles: Sparkles,
  glasses: Glasses,
  landmark: Landmark,
  compass: Compass,
  camera: Camera,
  music: Music,
  'pen-tool': PenTool,
  layers: Layers,
  folder: Folder,
  star: Star,
  zap: Zap,
  globe: Globe,
  cpu: Cpu,
  leaf: Leaf,
  target: Target,
};

export function BoardIcon({ name, size = 18 }: { name: string | null | undefined; size?: number }) {
  const Icon = (name && BOARD_ICONS[name]) || LayoutGrid;
  return <Icon size={size} strokeWidth={1.6} />;
}
