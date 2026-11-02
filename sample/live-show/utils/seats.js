// 座位图：13 排 × 18 列，中间第 9、10 列之间是过道；前两排两端各收 2 个座位（弧形内场）。
// 已售按场次 + 座位做确定性散列（同一场次每次打开一样），再叠加本机已买 / 待支付订单占用的座位。
const store = require('./store.js');
const ROWS = 13;
const COLS = 18;

function hash(str) {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = (h * 16777619) >>> 0; }
  return h;
}

function tierOfRow(show, row) {
  for (let i = 0; i < show.tiers.length; i++) {
    const t = show.tiers[i];
    if (t.rows && row >= t.rows[0] && row <= t.rows[1]) return t;
  }
  return show.tiers[show.tiers.length - 1];
}

function exists(row, col) {
  if (row <= 2 && (col <= 2 || col > COLS - 2)) return false;
  return true;
}

// 返回 { rows: [{ row, seats: [...] }], cols }，seats 里每个元素是一个座位或过道
function build(show, sessionId) {
  const taken = store.takenSeats(sessionId);
  const rows = [];
  for (let r = 1; r <= ROWS; r++) {
    const t = tierOfRow(show, r);
    const seats = [];
    let no = 0;
    for (let c = 1; c <= COLS; c++) {
      if (c === 10) seats.push({ id: 'aisle-' + r, gap: true });
      if (!exists(r, c)) { seats.push({ id: r + '-' + c, gap: true }); continue; }
      no += 1;
      const id = r + '-' + c;
      const sold = taken[id] || (hash(sessionId + ':' + id) % 100) < (r <= 5 ? 45 : 28);
      seats.push({ id, row: r, no, tier: t.id, price: t.price, color: t.color, sold: !!sold, label: r + '排' + no + '座' });
    }
    rows.push({ row: r, seats });
  }
  return { rows, cols: COLS + 1 };
}

module.exports = { build, ROWS, COLS };
