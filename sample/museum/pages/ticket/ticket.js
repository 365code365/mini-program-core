// 预约凭证：入馆码（按预约号生成的 21×21 点阵，三个定位角 + 伪随机数据区，闸机端按预约号核验）、
// 参观人（姓名 / 证件已打码）、取消预约、到馆指引（wx.openLocation，失败显示地址并可复制）
const U = require('../../utils/util.js');

const ADDR = { name: '云川博物院', address: '云川市青石区文华路 88 号（地铁 2 号线文华路站 B 口）', latitude: 30.2741, longitude: 120.1551 };

function matrix(code) {
  const n = 21, m = [];
  let h = 2166136261;
  for (const c of code) h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
  for (let y = 0; y < n; y++) {
    const row = [];
    for (let x = 0; x < n; x++) { h = (Math.imul(h, 1103515245) + 12345) >>> 0; row.push((h >>> 16) & 1); }
    m.push(row);
  }
  const finder = (ox, oy) => {
    for (let y = -1; y <= 7; y++) for (let x = -1; x <= 7; x++) {
      const X = ox + x, Y = oy + y;
      if (X < 0 || Y < 0 || X >= n || Y >= n) continue;
      const edge = x === 0 || x === 6 || y === 0 || y === 6, core = x >= 2 && x <= 4 && y >= 2 && y <= 4;
      m[Y][X] = x >= 0 && x <= 6 && y >= 0 && y <= 6 && (edge || core) ? 1 : 0;
    }
  };
  finder(0, 0); finder(n - 7, 0); finder(0, n - 7);
  for (let i = 8; i < n - 8; i++) { m[6][i] = i % 2 ? 0 : 1; m[i][6] = i % 2 ? 0 : 1; }
  return m.map((r, y) => ({ k: 'r' + y, cells: r.map((v, x) => ({ k: x, v })) }));
}

Page({
  data: { b: null, qr: [], isNew: false, addr: ADDR },
  onLoad(q) {
    this.bid = q.id;
    this.setData({ isNew: q.new === '1' });
    this.load();
  },
  load() {
    const b = U.load('bookings', []).find(x => x.id === this.bid);
    if (!b) { this.setData({ b: null }); return; }
    const wk = U.WEEK[new Date(b.date.replace(/-/g, '/')).getDay()];
    this.setData({ b: Object.assign({ wk }, b), qr: matrix(b.code) });
  },
  cancel() {
    wx.showModal({
      title: '取消预约', content: '取消后名额将释放给其他观众，如需参观可重新预约。确定取消吗？',
      confirmText: '确定取消', cancelText: '保留', confirmColor: '#c0392b',
      success: r => {
        if (!r.confirm) return;
        const list = U.load('bookings', []);
        const b = list.find(x => x.id === this.bid);
        b.status = '已取消';
        b.cancelAt = Date.now();
        U.save('bookings', list);
        this.load();
        wx.showToast({ title: '已取消预约', icon: 'none' });
      }
    });
  },
  guide() {
    wx.openLocation(Object.assign({ scale: 16 }, ADDR, {
      fail: () => wx.showModal({
        title: '到馆指引', content: ADDR.address + '。馆内停车场 120 个车位，节假日建议公共交通出行。',
        confirmText: '复制地址', cancelText: '关闭',
        success: r => { if (r.confirm) wx.setClipboardData({ data: ADDR.address }); }
      })
    }));
  },
  copyCode() { wx.setClipboardData({ data: this.data.b.code }); },
  home() { wx.switchTab({ url: '/pages/index/index' }); },
  rebook() { wx.navigateTo({ url: '/pages/booking/booking' }); }
});
