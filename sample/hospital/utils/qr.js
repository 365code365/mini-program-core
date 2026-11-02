// 电子凭证上的二维码：按号码生成固定点阵（CSS 画，不用 canvas）。9×9 格，三个角留给定位框
function qrDots(id, cell, pad) {
  const c = cell || 20;
  const p = pad || 16;
  let h = 0;
  const s = String(id);
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003;
  const out = [];
  for (let r = 0; r < 9; r++) {
    for (let k = 0; k < 9; k++) {
      const corner = (r < 3 && k < 3) || (r < 3 && k > 5) || (r > 5 && k < 3);
      h = (h * 1103 + 12345) % 100003;
      if (!corner && h % 2) out.push({ k: r * 9 + k, x: p + k * c, y: p + r * c });
    }
  }
  return out;
}
module.exports = { qrDots };
