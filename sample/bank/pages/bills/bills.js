const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

const PAGE = 15;

function monthsAgo(n) {
  const d = new Date();
  return M.dateStr(new Date(d.getFullYear(), d.getMonth() - n, 1).getTime());
}

Page({
  data: {
    types: [{ k: 'all', t: '全部' }, { k: 'in', t: '收入' }, { k: 'out', t: '支出' }],
    type: 'all', typeName: '交易', acctNames: [], acctIdx: 0,
    start: '', end: '', today: '', groups: [], count: 0, shown: 0, moreText: '', inText: '0.00', outText: '0.00'
  },

  onLoad() {
    const today = M.dateStr(Date.now());
    this.setData({ today, end: today, start: monthsAgo(2) });
  },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.accounts = S.accounts();
    this.setData({ acctNames: ['全部账户'].concat(this.accounts.map(a => a.name)) });
    this.rebuild(true);
  },

  // 先按条件筛出全部，再按 shown 截一页分组：月份小计用全部数据算，翻页不改小计
  rebuild(resetPage) {
    const d = this.data;
    const s = new Date(d.start.replace(/-/g, '/')).getTime();
    const e = new Date(d.end.replace(/-/g, '/')).getTime() + 86400000 - 1;
    const acctId = d.acctIdx > 0 ? this.accounts[d.acctIdx - 1].id : '';
    const names = {};
    this.accounts.forEach(a => { names[a.id] = a.name; });
    const all = S.bills().filter(b => b.time >= s && b.time <= e && (d.type === 'all' || b.type === d.type) && (!acctId || b.acct === acctId))
      .sort((a, b) => b.time - a.time);
    const shown = Math.min(all.length, resetPage ? PAGE : Math.max(d.shown, PAGE));
    const totals = {};
    let inSum = 0;
    let outSum = 0;
    all.forEach(b => {
      const key = M.dateStr(b.time).slice(0, 7);
      const t = totals[key] = totals[key] || { in: 0, out: 0 };
      t[b.type] += b.amount;
      if (b.type === 'in') inSum += b.amount; else outSum += b.amount;
    });
    const groups = [];
    all.slice(0, shown).forEach(b => {
      const key = M.dateStr(b.time).slice(0, 7);
      let g = groups[groups.length - 1];
      if (!g || g.key !== key) {
        g = { key, title: key.slice(0, 4) + '年' + Number(key.slice(5)) + '月', inText: M.fmt(totals[key].in), outText: M.fmt(totals[key].out), items: [] };
        groups.push(g);
      }
      g.items.push({
        id: b.id, title: b.title, type: b.type, g: b.cat.slice(0, 1), acctName: names[b.acct] || '',
        timeText: M.timeStr(b.time).slice(5), amtText: (b.type === 'in' ? '+' : '-') + M.fmt(b.amount)
      });
    });
    this.setData({
      groups, count: all.length, shown, inText: M.fmt(inSum), outText: M.fmt(outSum),
      moreText: shown < all.length ? '上拉加载更多（已显示 ' + shown + '/' + all.length + '）' : '没有更多了',
      typeName: d.type === 'in' ? '收入' : d.type === 'out' ? '支出' : '交易'
    });
  },

  onType(e) {
    this.setData({ type: e.currentTarget.dataset.k });
    this.rebuild(true);
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },
  onAcct(e) { this.setData({ acctIdx: Number(e.detail.value) }); this.rebuild(true); },
  // 选择器的 start/end 引擎不一定卡得住（实测能选到 end 之后），这里再夹一次到今天
  clampDay(v) { return v > this.data.today ? this.data.today : v; },
  onStart(e) {
    let start = this.clampDay(e.detail.value);
    let end = this.data.end;
    if (start > end) { const t = start; start = end; end = t; wx.showToast({ title: '起止日期已对调', icon: 'none' }); }
    this.setData({ start, end });
    this.rebuild(true);
  },
  onEnd(e) {
    let end = this.clampDay(e.detail.value);
    let start = this.data.start;
    if (start > end) { const t = start; start = end; end = t; wx.showToast({ title: '起止日期已对调', icon: 'none' }); }
    this.setData({ start, end });
    this.rebuild(true);
  },
  resetRange() { this.setData({ start: monthsAgo(2), end: this.data.today }); this.rebuild(true); },

  onReachBottom() {
    if (this.data.shown >= this.data.count || this.loading) return;
    this.loading = true;
    this.setData({ moreText: '加载中…' });
    setTimeout(() => {
      this.loading = false;
      this.setData({ shown: this.data.shown + PAGE });
      this.rebuild(false);
    }, 400);
  },

  onPullDownRefresh() {
    setTimeout(() => {
      this.rebuild(true);
      wx.stopPullDownRefresh();
      wx.showToast({ title: '已刷新', icon: 'none' });
    }, 500);
  },

  open(e) { wx.navigateTo({ url: '/pages/bill-detail/bill-detail?id=' + e.currentTarget.dataset.id }); }
});
