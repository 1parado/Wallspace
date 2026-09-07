import type { WallpaperItem } from '../types';
import type { SortMode } from '../stores/ui';

/** 集合页额外支持 custom = 保持用户拖拽整理的顺序 */
export type CollSortMode = 'custom' | SortMode;

/** 种子随机数（mulberry32）：同一种子序列稳定，换种子即重新洗牌 */
export function seededRandom(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * 排序壁纸条目。返回新数组，不修改入参。
 * mode = custom 时原样返回（保持集合自定义顺序）。
 */
export function sortItems(
  items: WallpaperItem[],
  mode: CollSortMode,
  seed = 0
): WallpaperItem[] {
  switch (mode) {
    case 'custom':
      return items;
    case 'oldest':
      return [...items].sort((a, b) => a.createdAt - b.createdAt);
    case 'name':
      return [...items].sort((a, b) =>
        a.title.localeCompare(b.title, undefined, { sensitivity: 'base' })
      );
    case 'resolution':
      return [...items].sort((a, b) => b.width * b.height - a.width * a.height);
    case 'random': {
      const arr = [...items];
      const rnd = seededRandom(seed || 1);
      for (let i = arr.length - 1; i > 0; i--) {
        const j = Math.floor(rnd() * (i + 1));
        [arr[i], arr[j]] = [arr[j], arr[i]];
      }
      return arr;
    }
    default:
      return [...items].sort((a, b) => b.createdAt - a.createdAt);
  }
}
