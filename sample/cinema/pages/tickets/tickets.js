// 我的票：按状态分栏；待观影的展开看取票码，退票 / 改签 / 导航；看过的写影评；点票面进详情
const store = require('../../utils/store.js');
const ops = require('../../utils/ops.js');
const { qrMatrix } = require('../../utils/qr.js');
const TABS = [{ key: 'all', name: '全部' }, { key: 'unused', name: '待观影' }, { key: 'used', name: '已观影' }, { key: 'refunded', name: '已退票' }];
Page({
  data: { tabs: TABS, tab: 'all', all: [], list: [], open: '', qr: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    // 别的页面要求切到某一栏（「我的」里点「看过」、我的影评空态）
    const want = wx.getStorageSync(store.K.ticketTab);
    if (want) { wx.removeStorageSync(store.K.ticketTab); this.setData({ tab: want }); }
    this.load();
  },
  load() {
    const all = store.tickets();
    const tab = this.data.tab;
    const list = tab === 'all' ? all : all.filter(t => t.status === tab);
    const first = list.find(t => t.status === 'unused');
    const keep = this.data.open && list.some(t => t.id === this.data.open && t.status === 'unused');
    const open = keep ? this.data.open : (first ? first.id : '');
    const tabs = TABS.map(x => Object.assign({}, x, { n: x.key === 'all' ? all.length : all.filter(t => t.status === x.key).length }));
    this.setData({ all, list, open, tabs });
    this.buildQr();
  },
  buildQr() {
    const t = this.data.list.find(x => x.id === this.data.open);
    this.setData({ qr: t ? qrMatrix(t.code) : [] });
  },
  find(e) { return this.data.list.find(x => x.id === e.currentTarget.dataset.id); },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.key, open: '' }); this.load(); },
  onDetail(e) { wx.navigateTo({ url: '/pages/order/order?id=' + e.currentTarget.dataset.id }); },
  // 票根下沿：待观影展开 / 收起取票码，看过的去写影评
  onToggle(e) {
    const t = this.find(e);
    if (t.status === 'used') { this.review(t); return; }
    if (t.status === 'refunded') { this.onDetail(e); return; }
    this.setData({ open: this.data.open === t.id ? '' : t.id });
    this.buildQr();
  },
  review(t) {
    const r = store.reviewOf(t.id);
    wx.navigateTo({ url: '/pages/review/review?mid=' + t.mid + '&tid=' + t.id + (r ? '&id=' + r.id : '') });
  },
  onRefund(e) { ops.refund(this.find(e), () => this.load()); },
  onChange(e) { ops.change(this.find(e), () => this.load()); },
  onNav(e) { ops.nav(this.find(e)); },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
