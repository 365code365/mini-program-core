// 按取票码算一张 21×21 的示意二维码（三个定位角 + 时序线 + 伪随机数据位）
const N = 21;
function qrMatrix(code) {
  let h = 0;
  for (let i = 0; i < code.length; i++) h = (h * 131 + code.charCodeAt(i)) >>> 0;
  const rnd = () => { h = (h * 1103515245 + 12345) >>> 0; return (h >>> 16) & 1; };
  const m = [];
  for (let r = 0; r < N; r++) { m.push([]); for (let c = 0; c < N; c++) m[r].push(rnd()); }
  const finder = (r0, c0) => {
    for (let r = -1; r <= 7; r++) for (let c = -1; c <= 7; c++) {
      const rr = r0 + r, cc = c0 + c;
      if (rr < 0 || cc < 0 || rr >= N || cc >= N) continue;
      const ring = Math.max(Math.abs(r - 3), Math.abs(c - 3));
      m[rr][cc] = (r < 0 || c < 0 || r > 6 || c > 6) ? 0 : (ring === 3 || ring <= 1 ? 1 : 0);
    }
  };
  finder(0, 0); finder(0, N - 7); finder(N - 7, 0);
  for (let i = 8; i < N - 8; i++) { m[6][i] = i % 2 === 0 ? 1 : 0; m[i][6] = i % 2 === 0 ? 1 : 0; }
  return m;
}
module.exports = { qrMatrix };
