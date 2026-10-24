const app = getApp();
const N = 21;
const SEED = [
  { id: 'TK-seed-1', title: '长河落日', poster: '/images/m1.jpg', day: '10月5日', time: '19:30', cinema: '上影国际影城（新天地店）', hall: '杜比厅', seats: ['7排8座', '7排9座'], code: '58203716', total: 136, status: 'used' }
];

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

Page({
  data: { list: [], open: '', qr: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const list = app.getTickets().concat(SEED).map(t => Object.assign({}, t, {
      seatText: (t.seats || []).join(' '),
      codeText: t.code.slice(0, 4) + ' ' + t.code.slice(4)
    }));
    const first = list.find(t => t.status === 'unused');
    const open = this.data.open && list.some(t => t.id === this.data.open) ? this.data.open : (first ? first.id : '');
    this.setData({ list, open });
    this.buildQr();
  },
  buildQr() {
    const t = this.data.list.find(x => x.id === this.data.open);
    this.setData({ qr: t ? qrMatrix(t.code) : [] });
  },
  onToggle(e) {
    const id = e.currentTarget.dataset.id;
    const t = this.data.list.find(x => x.id === id);
    if (t.status !== 'unused') { wx.showToast({ title: '影评功能即将上线', icon: 'none' }); return; }
    this.setData({ open: this.data.open === id ? '' : id });
    this.buildQr();
  },
  onRefund(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认退票', content: '退票将收取 ¥2/张 手续费，票款 1-3 个工作日原路退回。', confirmText: '确认退票', confirmColor: '#E8243F',
      success: (r) => {
        if (!r.confirm) return;
        app.saveTickets(app.getTickets().filter(t => t.id !== id));
        wx.showToast({ title: '退票成功', icon: 'success' });
        this.onShow();
      }
    });
  },
  onChange() { wx.showActionSheet({ itemList: ['改到今天 21:35', '改到明天 19:10', '改到明天 21:35'], success: () => wx.showToast({ title: '改签成功', icon: 'success' }) }); },
  onNav() { wx.showToast({ title: '已复制影院地址', icon: 'none' }); },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
