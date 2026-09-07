/**
 * categoryTree.ts —— 层级子分类支持。
 *
 * 约定：category 字符串用 '/' 表示层级，如 'Nature/Mountains'；
 * 单段（'Nature'）即顶层分类，完全向后兼容旧数据。
 * 匹配规则：选中 'Nature' 时包含其所有后代（前缀匹配）。
 */

import type { WallpaperItem } from '../types';

export interface CatNode {
  /** 完整路径，如 'Nature' 或 'Nature/Mountains' */
  key: string;
  /** 显示名（最后一段） */
  name: string;
  /** 该节点及所有后代的条目数 */
  count: number;
  /** 仅该层（不含后代）的条目数 */
  own: number;
  children: CatNode[];
}

/**
 * 是否命中分类过滤：
 * - null  = 全部
 * - ''    = 未分类（category 为空）
 * - 其他  = 精确或前缀（父分类包含其所有后代）
 */
export function matchCategory(
  itemCat: string | null | undefined,
  filter: string | null
): boolean {
  if (filter === null) return true;
  if (filter === '') return !itemCat?.trim();
  const c = itemCat ?? '';
  return c === filter || c.startsWith(filter + '/');
}

/**
 * 统一的分类显示名：
 * - 未分类（null/''/空白）→ uncategorizedLabel
 * - 顶层知名分类（cat.xxx 有翻译）→ 翻译名
 * - 子分类 / 自定义分类 → 最后一段原名（保留大小写）
 */
export function displayCategory(
  key: string | null | undefined,
  t: (k: string) => string,
  uncategorizedLabel: string
): string {
  const c = key?.trim();
  if (!c) return uncategorizedLabel;
  const segs = c.split('/');
  const last = segs[segs.length - 1];
  if (segs.length === 1) {
    const k = 'cat.' + c.toLowerCase();
    // t() 对缺失键回退为键的最后一段（即小写原名），据此判断是否有真实翻译
    const tr = t(k);
    if (tr !== c.toLowerCase()) return tr;
  }
  return last;
}

function parentOf(path: string): string | null {
  return path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : null;
}

/** 从库内条目构建分类树（只含有内容的节点），同层按数量降序 */
export function buildCategoryTree(items: WallpaperItem[]): CatNode[] {
  const root: CatNode = { key: '', name: '', count: 0, own: 0, children: [] };
  const index = new Map<string, CatNode>();

  const ensure = (path: string): CatNode => {
    const found = index.get(path);
    if (found) return found;
    const segs = path.split('/');
    const parent = segs.length > 1 ? ensure(segs.slice(0, -1).join('/')) : root;
    const node: CatNode = {
      key: path,
      name: segs[segs.length - 1],
      count: 0,
      own: 0,
      children: [],
    };
    parent.children.push(node);
    index.set(path, node);
    return node;
  };

  for (const it of items) {
    const c = it.category?.trim();
    if (!c) continue;
    ensure(c);
    // 沿祖先链 +1
    let p: string | null = c;
    while (p) {
      index.get(p)!.count++;
      p = parentOf(p);
    }
  }

  const fixOwn = (n: CatNode) => {
    n.own = n.count;
    for (const ch of n.children) {
      fixOwn(ch);
      n.own -= ch.count;
    }
  };
  for (const ch of root.children) fixOwn(ch);

  const sortRec = (n: CatNode) => {
    n.children.sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
    n.children.forEach(sortRec);
  };
  sortRec(root);

  return root.children;
}
