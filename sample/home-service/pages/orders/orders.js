const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STAFF, CANCEL_REASONS } = require('../../utils/data.js');
// 订单列表：四个状态 tab + 全部。卡片是 order-card 组件，按钮 triggerEvent('action', …, {bubbles, composed})，
// 这里在外层 view 上用 bind:action 统一接（不必给每张卡片都绑一次）。
const TABS = [
  { key: '', text: '全部' }, { key: 'waiting', text: '待上门' }, { key: 'serving', text: '服务中' },
  { key: 'review', text: '待评价' }, { key: 'done', text: '已完成' }
];
function view(o) {
  const full = o.addr.full || '';
  return Object.assign({}, o, {
    specText: o.specName + ' × ' + o.qty + o.unit,
    addrShort: full.length > 20 ? full.slice(0, 20) + '…' : full
  });
}
Page({
  data: { tabs: TABS, tab: '', list: [], counts: {}, loading: true },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    // 「我的」页点状态入口时经 globalData 带过来（switchTab 不能带 query）
    const app = getApp();
    if (app.globalData.orderTab !== '') {
      this.setData({ tab: app.globalData.orderTab === 'all' ? '' : app.globalData.orderTab });
      app.globalData.orderTab = '';
    }
    this.load();
  },
  onPullDownRefresh() { this.load(); wx.stopPullDownRefresh(); },
  load() {
    this.setData({ loading: true });
    const all = db.getOrders();
    const counts = {};
    all.forEach((o) => { counts[o.status] = (counts[o.status] || 0) + 1; });
    const list = all.filter((o) => !this.data.tab || o.status === this.data.tab).map(view);
    this.setData({ list, counts, loading: false });
  },
  switchTab(e) {
    const tab = e.currentTarget.dataset.k;
    if (tab === this.data.tab) return;
    this.setData({ tab });
    this.load();
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  // order-card 冒泡上来的事件：e.detail = { type, id }
  onAction(e) {
    const { type, id } = e.detail;
    const o = db.getOrder(id);
    if (!o) return;
    const staff = STAFF.find((s) => s.id === o.staffId);
    if (type === 'open') wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + id });
    else if (type === 'call') util.callPhone(staff.phone, staff.name + '师傅');
    else if (type === 'review') wx.navigateTo({ url: '/pages/review/review?id=' + id });
    else if (type === 'again') wx.navigateTo({ url: '/pages/service/service?id=' + o.svcId });
    else if (type === 'cancel') this.cancel(o);
    else if (type === 'finish') this.finish(o);
    else if (type === 'delete') this.remove(o);
  },
  cancel(o) {
    // 列表里只给固定原因；要写「其他原因」进详情页的取消面板
    const reasons = CANCEL_REASONS.slice(0, CANCEL_REASONS.length - 1);
    wx.showActionSheet({
      itemList: reasons,
      success: (r) => {
        const reason = reasons[r.tapIndex];
        if (o.couponId) db.setCouponStatus(o.couponId, 'unused');
        db.saveOrder(db.pushTimeline(Object.assign(o, { status: 'cancelled', cancelReason: reason }), '已取消：' + reason));
        wx.showToast({ title: '订单已取消', icon: 'success' });
        this.load();
      }
    });
  },
  finish(o) {
    wx.showModal({
      title: '确认服务完成', content: '请检查服务效果，确认后支付 ¥' + o.pay + '。', confirmText: '确认完成',
      success: (r) => {
        if (!r.confirm) return;
        // 微信支付不可用（桌面端立刻 fail）：线下扫师傅收款码，照样完成订单
        wx.requestPayment({
          timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: 'hg' + Date.now(), package: 'prepay_id=' + o.id, signType: 'RSA', paySign: 'demo',
          complete: () => {
            db.saveOrder(db.pushTimeline(Object.assign(o, { status: 'review' }), '服务完成，已付款 ¥' + o.pay));
            this.load();
          }
        });
      }
    });
  },
  remove(o) {
    wx.showModal({
      title: '删除订单', content: '删除后不能恢复', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { db.removeOrder(o.id); this.load(); } }
    });
  }
});
