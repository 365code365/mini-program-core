const S = require('../../utils/store.js');

const TABS = [
  { k: 'all', t: '全部' },
  { k: 'ing', t: '进行中' },
  { k: 'done', t: '已完成' },
  { k: 'canceled', t: '已取消' }
];
const PAGE = 6;

function match(o, k) {
  if (k === 'all') return true;
  if (k === 'ing') return o.status === 'unpaid' || o.status === 'paid';
  return o.status === k;
}

Page({
  data: { tabs: TABS, tab: 'all', list: [], hasMore: true, total: 0, refreshedAt: '' },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.reload();
    clearInterval(this.timer);
    // 列表里有进行中的订单时，每 2 秒同步一次状态（倒计时 / 出餐阶段）
    this.timer = setInterval(() => { if (this.hasActive) this.reload(true); }, 2000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  view(o) {
    const n = o.items.reduce((s, i) => s + i.qty, 0);
    let left = '';
    if (o.status === 'unpaid') {
      const sec = Math.max(0, Math.ceil((o.deadline - Date.now()) / 1000));
      left = '剩 ' + S.pad(Math.floor(sec / 60)) + ':' + S.pad(sec % 60);
    }
    return {
      id: o.id, no: o.no, table: o.table, status: o.status, statusText: S.statusText(o), time: S.fmtTime(o.createdAt),
      names: o.items.slice(0, 3).map(i => i.name).join('、') + (o.items.length > 3 ? ' 等' : ''),
      thumbs: o.items.slice(0, 4).map(i => ({ bg: i.bg, ch: i.ch, k: i.key })), count: n, payText: S.yuan(o.pay), left
    };
  },
  // keep=true 时保留已加载的页数（定时刷新不把用户翻过的页收回去）
  reload(keep) {
    const all = S.getOrders().filter(o => match(o, this.data.tab));
    this.hasActive = all.some(o => o.status === 'unpaid' || o.status === 'paid');
    const n = keep ? Math.max(PAGE, this.data.list.length) : PAGE;
    this.all = all;
    this.setData({ list: all.slice(0, n).map(o => this.view(o)), hasMore: all.length > n, total: all.length });
  },
  onTab(e) {
    const k = e.currentTarget.dataset.k;
    if (k === this.data.tab) return;
    this.setData({ tab: k });
    this.reload();
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },
  onPullDownRefresh() {
    setTimeout(() => {
      this.reload();
      const d = new Date();
      this.setData({ refreshedAt: S.pad(d.getHours()) + ':' + S.pad(d.getMinutes()) + ':' + S.pad(d.getSeconds()) });
      wx.stopPullDownRefresh();
      wx.showToast({ title: '已刷新', icon: 'none' });
    }, 600);
  },
  onReachBottom() {
    if (!this.data.hasMore || this.loading) return;
    this.loading = true;
    setTimeout(() => {
      const n = this.data.list.length + PAGE;
      this.setData({ list: this.all.slice(0, n).map(o => this.view(o)), hasMore: this.all.length > n });
      this.loading = false;
    }, 300);
  },
  open(e) { wx.navigateTo({ url: '/pages/progress/progress?id=' + e.currentTarget.dataset.id }); },
  pay(e) { wx.navigateTo({ url: '/pages/pay/pay?id=' + e.currentTarget.dataset.id }); },
  again(e) {
    const o = S.getOrder(e.currentTarget.dataset.id);
    S.reorder(o);
    wx.showToast({ title: '已加入购物车', icon: 'none' });
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
