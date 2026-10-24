const app = getApp();
const ROWS = 9, COLS = 14;
const AISLE = [3, 12]; // 这两列之前留过道

function hash(s) { let h = 7; for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003; return h; }

Page({
  data: { rows: [], selected: [], total: 0, m: null, c: null, pick: {} },
  onLoad(q) {
    const pick = wx.getStorageSync('cinema_pick') || { mid: q.mid || 'm1', cid: q.cid || 'c1', day: '今天', time: q.time || '19:10', lang: '国语IMAX', hall: 'IMAX厅', price: 69, end: '21:38' };
    const m = app.getMovie(pick.mid), c = app.getCinema(pick.cid);
    this.setData({ pick, m, c });
    wx.setNavigationBarTitle({ title: c.name.replace(/（.*）/, '') });
    this.sold = {};
    const seed = hash(pick.mid + pick.cid + pick.time);
    for (let r = 1; r <= ROWS; r++) for (let col = 1; col <= COLS; col++) {
      const v = (seed + r * 131 + col * 71 + r * col * 17) % 100;
      const centerBias = (r >= 4 && r <= 7 && col >= 5 && col <= 10) ? 22 : 0;
      if (v < 18 + centerBias) this.sold[r + '-' + col] = true;
    }
    this.sel = [];
    this.render();
  },
  render() {
    const rows = [];
    for (let r = 1; r <= ROWS; r++) {
      const seats = [];
      for (let col = 1; col <= COLS; col++) {
        if (AISLE.indexOf(col) >= 0) seats.push({ key: 'g' + col, state: 'gap' });
        const k = r + '-' + col;
        seats.push({ key: k, c: col, state: this.sold[k] ? 'sold' : (this.sel.indexOf(k) >= 0 ? 'sel' : 'empty'), best: r >= 4 && r <= 7 && col >= 5 && col <= 10 });
      }
      rows.push({ r, seats });
    }
    const selected = this.sel.map(k => { const [r, c] = k.split('-').map(Number); return { key: k, r, c }; })
      .sort((a, b) => a.r - b.r || a.c - b.c);
    this.setData({ rows, selected, total: selected.length * this.data.pick.price });
  },
  onSeat(e) {
    const k = e.currentTarget.dataset.r + '-' + e.currentTarget.dataset.c;
    if (this.sold[k]) return;
    const i = this.sel.indexOf(k);
    if (i >= 0) this.sel.splice(i, 1);
    else {
      if (this.sel.length >= 4) { wx.showToast({ title: '一次最多选 4 个座位', icon: 'none' }); return; }
      this.sel.push(k);
    }
    this.render();
  },
  onRecommend(e) {
    const n = e.currentTarget.dataset.n;
    const order = [5, 6, 4, 7, 3, 8, 2, 9, 1];
    for (const r of order) {
      for (let start = 5; start >= 1; start--) {
        for (const s of [7 - Math.floor(n / 2) + (5 - start), 7 - Math.floor(n / 2) - (5 - start)]) {
          if (s < 1 || s + n - 1 > COLS) continue;
          const keys = [];
          let ok = true;
          for (let c = s; c < s + n; c++) { if (this.sold[r + '-' + c] || (c > s && AISLE.indexOf(c) >= 0)) { ok = false; break; } keys.push(r + '-' + c); }
          if (ok) { this.sel = keys; this.render(); return; }
        }
      }
    }
    wx.showToast({ title: '没有连座了，请手动选择', icon: 'none' });
  },
  onConfirm() {
    if (!this.data.selected.length) { wx.showToast({ title: '请先选择座位', icon: 'none' }); return; }
    const p = Object.assign({}, this.data.pick, { seats: this.data.selected.map(s => s.r + '排' + s.c + '座'), total: this.data.total });
    wx.setStorageSync('cinema_pick', p);
    wx.navigateTo({ url: '/pages/confirm/confirm' });
  }
});
