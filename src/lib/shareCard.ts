// shareCard.ts —— 生成每日精选分享图卡（竖版 1080×1620，纯 canvas 合成）。
//
// 图片来源走 Rust 读原始字节 → Blob → createImageBitmap，
// 避免 asset 协议跨域导致的画布污染（toBlob SecurityError）。

import * as api from './api';
import type { WallpaperItem } from '../types';

const FONT = '"Segoe UI", "Microsoft YaHei", "PingFang SC", sans-serif';

function roundRectPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number
) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/** cover 模式铺满目标区域（可整体放大以规避模糊边缘） */
function drawCover(
  ctx: CanvasRenderingContext2D,
  bmp: ImageBitmap,
  w: number,
  h: number,
  scale = 1
) {
  const s = Math.max(w / bmp.width, h / bmp.height) * scale;
  const dw = bmp.width * s;
  const dh = bmp.height * s;
  ctx.drawImage(bmp, (w - dw) / 2, (h - dh) / 2, dw, dh);
}

function truncate(ctx: CanvasRenderingContext2D, text: string, maxW: number): string {
  if (ctx.measureText(text).width <= maxW) return text;
  let t = text;
  while (t.length > 1 && ctx.measureText(t + '…').width > maxW) {
    t = t.slice(0, -1);
  }
  return t + '…';
}

/** 合成分享卡并返回画布；由调用方负责 toBlob / 展示 */
export async function renderShareCard(item: WallpaperItem): Promise<HTMLCanvasElement> {
  const buf = await api.readBinaryFile(item.filePath);
  const blob = new Blob([buf]);
  const bmp = await createImageBitmap(blob);

  const W = 1080;
  const H = 1620;
  const canvas = document.createElement('canvas');
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('canvas 2d context unavailable');

  // 背景：cover 铺满 + 高斯模糊 + 暗化渐变
  ctx.filter = 'blur(48px)';
  drawCover(ctx, bmp, W, H, 1.2);
  ctx.filter = 'none';
  const grad = ctx.createLinearGradient(0, 0, 0, H);
  grad.addColorStop(0, 'rgba(0,0,0,0.30)');
  grad.addColorStop(0.55, 'rgba(0,0,0,0.42)');
  grad.addColorStop(1, 'rgba(0,0,0,0.72)');
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, W, H);

  // 主图：contain 居中 + 圆角裁切 + 微光描边
  const pad = 72;
  const maxW = W - pad * 2;
  const maxH = H - pad * 2 - 240;
  const scale = Math.min(maxW / bmp.width, maxH / bmp.height);
  const dw = bmp.width * scale;
  const dh = bmp.height * scale;
  const dx = (W - dw) / 2;
  const dy = pad + (maxH - dh) / 2 - 40;
  ctx.save();
  roundRectPath(ctx, dx, dy, dw, dh, 28);
  ctx.clip();
  ctx.drawImage(bmp, dx, dy, dw, dh);
  ctx.restore();
  ctx.strokeStyle = 'rgba(255,255,255,0.28)';
  ctx.lineWidth = 2;
  roundRectPath(ctx, dx, dy, dw, dh, 28);
  ctx.stroke();

  // 文案区：日期 / 标题 / 署名
  ctx.textAlign = 'center';
  ctx.fillStyle = 'rgba(255,255,255,0.85)';
  ctx.font = `600 30px ${FONT}`;
  ctx.fillText(new Date().toLocaleDateString(), W / 2, H - 168);

  ctx.fillStyle = '#ffffff';
  ctx.font = `600 46px ${FONT}`;
  ctx.fillText(truncate(ctx, item.title, W - pad * 2), W / 2, H - 102);

  ctx.fillStyle = 'rgba(255,255,255,0.55)';
  ctx.font = `400 22px ${FONT}`;
  ctx.fillText('Wallspace', W / 2, H - 52);

  bmp.close();
  return canvas;
}

/** 画布转 PNG Blob */
export function canvasToPng(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob failed'))), 'image/png')
  );
}

/** 画布转 JPEG Blob（体积小，适合落盘） */
export function canvasToJpeg(canvas: HTMLCanvasElement, quality = 0.92): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error('toBlob failed'))),
      'image/jpeg',
      quality
    )
  );
}
