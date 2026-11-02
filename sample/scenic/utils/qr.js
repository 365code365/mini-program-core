// 简化二维码：25×25 模块，按 QR 版本 2 的样子画三个定位角、定时线与校正图形，数据区由入园码哈希出的伪随机位填充。
// 闸机核验靠入园码与身份证，二维码只做展示（同一个码画出来永远一样）。
const N = 25;

function hash(s) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
  return h || 1;
}

function matrix(code) {
  const m = [], fixed = [];
  for (let y = 0; y < N; y++) { m.push(new Array(N).fill(0)); fixed.push(new Array(N).fill(false)); }
  const finder = (ox, oy) => {
    for (let y = -1; y <= 7; y++) for (let x = -1; x <= 7; x++) {
      const X = ox + x, Y = oy + y;
      if (X < 0 || Y < 0 || X >= N || Y >= N) continue;
      const edge = x === 0 || x === 6 || y === 0 || y === 6, core = x >= 2 && x <= 4 && y >= 2 && y <= 4;
      m[Y][X] = x >= 0 && x <= 6 && y >= 0 && y <= 6 && (edge || core) ? 1 : 0;
      fixed[Y][X] = true;
    }
  };
  finder(0, 0); finder(N - 7, 0); finder(0, N - 7);
  for (let i = 8; i < N - 8; i++) { m[6][i] = m[i][6] = i % 2 === 0 ? 1 : 0; fixed[6][i] = fixed[i][6] = true; }
  for (let y = -2; y <= 2; y++) for (let x = -2; x <= 2; x++) {
    const v = Math.max(Math.abs(x), Math.abs(y));
    m[18 + y][18 + x] = v === 1 ? 0 : 1; fixed[18 + y][18 + x] = true;
  }
  let s = hash(String(code));
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    if (fixed[y][x]) continue;
    s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0;
    m[y][x] = (s & 7) < 4 ? 1 : 0;
  }
  return m;
}

// 用旧版 canvas 上下文画到 size×size 的区域（留 1 格白边）
function draw(ctx, code, size, color) {
  const m = matrix(code), cell = size / (N + 2);
  ctx.setFillStyle('#ffffff');
  ctx.fillRect(0, 0, size, size);
  ctx.setFillStyle(color || '#1d2722');
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    if (m[y][x]) ctx.fillRect((x + 1) * cell, (y + 1) * cell, cell + 0.4, cell + 0.4);
  }
}

module.exports = { matrix, draw, N };
