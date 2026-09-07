// 色系（hue family）：把任意 hex 归入 12 个感知色系之一，用于按色调筛选壁纸。
// 相比精确 HEX 匹配，色系匹配能覆盖同一色调下的深浅变化。

export interface ColorFamily {
  id: string;
  /** 展示用代表色 */
  hex: string;
  labelKey: string;
}

export const COLOR_FAMILIES: ColorFamily[] = [
  { id: 'red', hex: '#e5484d', labelKey: 'facets.family.red' },
  { id: 'orange', hex: '#f76b15', labelKey: 'facets.family.orange' },
  { id: 'yellow', hex: '#ffb224', labelKey: 'facets.family.yellow' },
  { id: 'green', hex: '#3e9b4f', labelKey: 'facets.family.green' },
  { id: 'cyan', hex: '#12a594', labelKey: 'facets.family.cyan' },
  { id: 'blue', hex: '#0090ff', labelKey: 'facets.family.blue' },
  { id: 'purple', hex: '#8e4ec6', labelKey: 'facets.family.purple' },
  { id: 'pink', hex: '#e93d82', labelKey: 'facets.family.pink' },
  { id: 'brown', hex: '#ad7f58', labelKey: 'facets.family.brown' },
  { id: 'black', hex: '#2b2b2b', labelKey: 'facets.family.black' },
  { id: 'white', hex: '#f2f0ef', labelKey: 'facets.family.white' },
  { id: 'gray', hex: '#8f8f8f', labelKey: 'facets.family.gray' },
];

/** hex → [h(0-360), s(0-1), v(0-1)]；解析失败返回 null */
function hexToHsv(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d > 0) {
    if (max === r) h = (g - b) / d;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h = (h * 60 + 360) % 360;
  }
  return [h, max === 0 ? 0 : d / max, max];
}

/** 把 hex 归入色系；无法解析返回 null */
export function familyOfHex(hex: string): string | null {
  const hsv = hexToHsv(hex);
  if (!hsv) return null;
  const [h, s, v] = hsv;
  // 无彩色系：饱和度低时按明度分黑/白/灰
  if (s < 0.18) {
    if (v < 0.22) return 'black';
    if (v > 0.82) return 'white';
    return 'gray';
  }
  // 棕 = 暗橙/暗黄（暗红仍归红系）
  if (h >= 15 && h < 48 && v < 0.62) return 'brown';
  if (h < 15 || h >= 345) return 'red';
  if (h < 48) return 'orange';
  if (h < 70) return 'yellow';
  if (h < 165) return 'green';
  if (h < 195) return 'cyan';
  if (h < 255) return 'blue';
  if (h < 290) return 'purple';
  return 'pink';
}

/** 按色系列出每个条目命中的色系并计数（一图可命中多系，但每系只计一次） */
export function countByFamily(items: { palette?: string[] }[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const it of items) {
    const fams = new Set<string>();
    for (const c of it.palette ?? []) {
      const f = familyOfHex(c);
      if (f) fams.add(f);
    }
    for (const f of fams) counts.set(f, (counts.get(f) ?? 0) + 1);
  }
  return counts;
}
