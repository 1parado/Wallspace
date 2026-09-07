/**
 * autoTag.ts —— 智能自动分类（关键词规则起步，后续可接轻量模型）。
 *
 * - guessCategory(prompt): 根据提示词给出生图分类建议（无命中返回 null，由调用方回退到记忆值）
 * - suggestTags(prompt):   产出主题 + mood/style 标签（含 dark/light/vibrant/minimal/cyberpunk/anime 等维度）
 *
 * 规则表集中维护，命中全部规则后按得分排序取最优分类。
 */

interface Rule {
  /** 关键词（小写，按包含匹配；中文直接匹配子串） */
  words: string[];
  /** 命中加 1 分 */
  category?: string;
  /** 命中时注入的标签 */
  tags?: string[];
}

const RULES: Rule[] = [
  // —— 主题分类 ——
  { category: 'Nature', tags: ['nature'], words: ['自然', '风景', '山水', '森林', '大海', '海洋', '山', '天空', '日落', '日出', '花', '树', '雪', '湖', '河流', 'nature', 'landscape', 'mountain', 'ocean', 'sea', 'forest', 'sky', 'sunset', 'sunrise', 'beach', 'river', 'cloud', 'flower', 'tree', 'snow'] },
  { category: 'Space', tags: ['space'], words: ['太空', '星云', '星空', '银河', '宇宙', '星球', '深空', '宇航', 'space', 'galaxy', 'nebula', 'cosmos', 'planet', 'star', 'astronaut', 'universe'] },
  { category: 'Abstract', tags: ['abstract'], words: ['抽象', '几何', '渐变', '流体', '图案', '纹理', '3d', 'abstract', 'geometry', 'geometric', 'gradient', 'fluid', 'pattern', 'texture', 'shape'] },
  { category: 'Cinematic', tags: ['cinematic'], words: ['电影', '电影感', '赛博朋克', '霓虹', '夜城', '都市', '街拍', '银翼杀手', 'cinematic', 'movie', 'film', 'cyberpunk', 'neon', 'blade runner', 'noir', 'city', 'street', 'urban'] },
  { category: 'Minimal', tags: ['minimal'], words: ['极简', '简洁', '简约', '克制', '留白', '单色', '线条', 'minimal', 'minimalist', 'clean', 'simple', 'line art', 'monochrome', 'negative space'] },

  // —— mood / style 标签 ——
  { tags: ['dark'], words: ['黑暗', '暗色', '深夜', '黑色', '阴影', 'dark', 'moody', 'black', 'shadow', 'night'] },
  { tags: ['light'], words: ['明亮', '浅色', '白色', '清晨', '柔和', 'light', 'bright', 'white', 'soft', 'pastel', 'airy'] },
  { tags: ['vibrant'], words: ['鲜艳', '绚丽', '多彩', '高饱和', ' vibrant', 'vibrant', 'colorful', 'saturated', 'bold'] },
  { tags: ['cyberpunk'], words: ['赛博朋克', '赛博', '机甲', '未来城市', 'cyberpunk', 'cyber', 'futuristic', 'mecha', 'dystopian'] },
  { tags: ['anime'], words: ['动漫', '二次元', '少女', '漫画', 'anime', 'manga', 'waifu', 'shoujo', 'ghibli'] },
  { tags: ['retro'], words: ['复古', '怀旧', '胶片', '老式', 'retro', 'vintage', 'film grain', '80s', '90s'] },
  { tags: ['anime', 'illustration'], words: ['插画', '插画风格', 'illustration', 'artwork', 'painting'] },
  { tags: ['photography'], words: ['摄影', '实拍', '照片级', '超写实', 'photography', 'photo', 'photorealistic', 'hyperrealistic'] },
  { tags: ['icon', 'minimal'], words: ['图标', 'icon', 'logo', 'app icon'] },
  { tags: ['wallpaper'], words: ['壁纸', '桌面', 'wallpaper', 'desktop'] },
];

/** 每类标签上限，避免规则表把 tags 撑爆 */
const MAX_TAGS = 6;

export function suggestTags(prompt: string): string[] {
  const p = prompt.toLowerCase();
  if (!p.trim()) return [];
  const hit = new Set<string>();
  for (const rule of RULES) {
    if (!rule.tags) continue;
    if (rule.words.some((w) => p.includes(w))) {
      rule.tags.forEach((tg) => hit.add(tg));
    }
  }
  return [...hit].slice(0, MAX_TAGS);
}

/** 返回得分最高的分类；无任何命中时返回 null */
export function guessCategory(prompt: string): string | null {
  const p = prompt.toLowerCase();
  if (!p.trim()) return null;
  const scores = new Map<string, number>();
  for (const rule of RULES) {
    if (!rule.category) continue;
    const hits = rule.words.filter((w) => p.includes(w)).length;
    if (hits > 0) {
      scores.set(rule.category, (scores.get(rule.category) ?? 0) + hits);
    }
  }
  if (scores.size === 0) return null;
  return [...scores.entries()].sort((a, b) => b[1] - a[1])[0][0];
}

/** 分类 + 标签一次算齐，供生图/下载入口共用 */
export function autoClassify(prompt: string): { category: string | null; tags: string[] } {
  return { category: guessCategory(prompt), tags: suggestTags(prompt) };
}
