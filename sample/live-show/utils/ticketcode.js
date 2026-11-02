// 电子票码：在 canvas 2d 上画一个 25×25 的二维码样式票码（三个定位角 + 由票号散列出的数据点）。
// 只用于入场核验演示，不是标准 QR 编码；票号同时以文字显示在码下方，闸机可手输。
function hash(str, seed) {
  let h = seed >>> 0;
  for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
  return h;
}
function modules(code) {
  const N = 25, m = [];
  for (let y = 0; y < N; y++) { m.push([]); for (let x = 0; x < N; x++) m[y].push(false); }
  let h = hash(code, 2166136261);
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    h = Math.imul(h ^ (h >>> 13), 1274126177) >>> 0;
    m[y][x] = (h & 7) < 4;
  }
  const finder = (ox, oy) => {
    for (let y = -1; y < 8; y++) for (let x = -1; x < 8; x++) {
      const X = ox + x, Y = oy + y;
      if (X < 0 || Y < 0 || X >= N || Y >= N) continue;
      const ring = Math.max(Math.abs(x - 3), Math.abs(y - 3));
      m[Y][X] = x >= 0 && y >= 0 && x < 7 && y < 7 && ring !== 2;
    }
  };
  finder(0, 0); finder(N - 7, 0); finder(0, N - 7);
  return m;
}
// 画到 canvas 节点上（size 为逻辑像素边长）
function draw(canvas, size, code, dim) {
  const dpr = (wx.getWindowInfo().pixelRatio) || 2;
  canvas.width = size * dpr; canvas.height = size * dpr;
  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, size, size);
  const m = modules(code), N = m.length, pad = 8, cell = (size - pad * 2) / N;
  ctx.fillStyle = dim ? '#c6c7cc' : '#1d1d22';
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    if (m[y][x]) ctx.fillRect(pad + x * cell, pad + y * cell, Math.ceil(cell), Math.ceil(cell));
  }
}
module.exports = { draw };
