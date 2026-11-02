// 环形饼图：canvas type="2d"，按 dpr 放大画布像素
function draw(canvas, w, h, dpr, items, center) {
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);
  const ctx = canvas.getContext('2d');
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);
  const cx = w / 2, cy = h / 2, r = Math.min(w, h) / 2 - 6, inner = r * 0.62;
  const total = items.reduce((s, it) => s + it.value, 0) || 1;
  let a = -Math.PI / 2;
  items.forEach((it) => {
    const da = it.value / total * Math.PI * 2;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.arc(cx, cy, r, a, a + da);
    ctx.closePath();
    ctx.fillStyle = it.color;
    ctx.fill();
    a += da;
  });
  // 分隔白线
  a = -Math.PI / 2;
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 2;
  items.forEach((it) => {
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(a) * r, cy + Math.sin(a) * r);
    ctx.stroke();
    a += it.value / total * Math.PI * 2;
  });
  ctx.beginPath();
  ctx.arc(cx, cy, inner, 0, Math.PI * 2);
  ctx.fillStyle = '#ffffff';
  ctx.fill();
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = '#9a948c';
  ctx.font = '12px sans-serif';
  ctx.fillText(center.label, cx, cy - 14);
  ctx.fillStyle = '#2a2723';
  ctx.font = 'bold 22px sans-serif';
  ctx.fillText(center.value, cx, cy + 10);
}

module.exports = { draw };
