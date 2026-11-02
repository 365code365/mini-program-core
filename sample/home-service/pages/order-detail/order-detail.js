const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STAFF, CANCEL_REASONS, HOTLINE } = require('../../utils/data.js');

const STEPS = ['已下单', '已上门', '已完成', '已评价'];
const STEP_OF = { waiting: 0, serving: 1, review: 2, done: 3, cancelled: -1 };
const HEAD = {
  waiting: { t: '待上门', s: '师傅会按约定时间上门，上门前 30 分钟电话联系' },
  serving: { t: '服务中', s: '师傅正在服务，完成后请检查并确认' },
  review: { t: '待评价', s: '服务已完成，说说这次的体验吧' },
  done: { t: '已完成', s: '感谢使用好管家，24 小时内不满意可免费返工' },
  cancelled: { t: '已取消', s: '订单已取消，优惠券已退回' }
};

Page({
  data: { order: null, staff: null, steps: STEPS, step: 0, head: {}, reasons: CANCEL_REASONS, showCancel: false, reasonIdx: -1, other: '' },
  onLoad(q) { this.id = q.id; },
  onShow() { this.load(); },
  load() {
    const order = db.getOrder(this.id);
    if (!order) { this.setData({ order: null }); return; }
    this.setData({
      order, staff: STAFF.find((s) => s.id === order.staffId), step: STEP_OF[order.status], head: HEAD[order.status],
      timeline: order.timeline.slice().reverse().map((x, i) => Object.assign({ k: 'tl' + i }, x)),
      photoUrls: order.photos.map((p) => p.path),
      reviewTags: order.review && order.review.tags.length ? order.review.tags.join('、') : '未选标签'
    });
  },
  save(order, text) { db.saveOrder(text ? db.pushTimeline(order, text) : order); this.load(); },
  callStaff() { util.callPhone(this.data.staff.phone, this.data.staff.name + '师傅'); },
  callService() { util.callPhone(HOTLINE, '好管家客服'); },
  preview(e) { wx.previewImage({ current: e.currentTarget.dataset.src, urls: this.data.photoUrls }); },
  // ---------- 取消：底部面板选原因 ----------
  openCancel() { this.setData({ showCancel: true, reasonIdx: -1, other: '' }); },
  closeCancel() { this.setData({ showCancel: false }); },
  onReason(e) { this.setData({ reasonIdx: +e.detail.value }); },
  onOther(e) { this.setData({ other: e.detail.value }); },
  confirmCancel() {
    const i = this.data.reasonIdx;
    if (i < 0) { wx.showToast({ title: '请选择取消原因', icon: 'none' }); return; }
    const isOther = i === CANCEL_REASONS.length - 1;
    if (isOther && this.data.other.trim().length < 2) { wx.showToast({ title: '请简单说明原因', icon: 'none' }); return; }
    const reason = isOther ? this.data.other.trim() : CANCEL_REASONS[i];
    const o = Object.assign({}, this.data.order, { status: 'cancelled', cancelReason: reason });
    if (o.couponId) db.setCouponStatus(o.couponId, 'unused');
    this.setData({ showCancel: false });
    this.save(o, '已取消：' + reason);
    wx.showToast({ title: '订单已取消', icon: 'success' });
  },
  // ---------- 状态推进 ----------
  arrived() {
    wx.showModal({
      title: '确认师傅已上门', content: this.data.staff.name + '师傅已到达并出示工牌？确认后开始计时。',
      success: (r) => { if (r.confirm) this.save(Object.assign({}, this.data.order, { status: 'serving' }), '师傅已上门，开始服务'); }
    });
  },
  finish() {
    wx.showModal({
      title: '确认服务完成', content: '请检查服务效果，确认后支付 ¥' + this.data.order.pay + '。',
      confirmText: '确认完成',
      success: (r) => {
        if (!r.confirm) return;
        // 支付不可用时：线下扫师傅收款码，确认后照样完成订单
        wx.requestPayment({
          timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: 'hg' + Date.now(), package: 'prepay_id=' + this.data.order.id, signType: 'RSA', paySign: 'demo',
          complete: () => this.save(Object.assign({}, this.data.order, { status: 'review' }), '服务完成，已付款 ¥' + this.data.order.pay)
        });
      }
    });
  },
  review() { wx.navigateTo({ url: '/pages/review/review?id=' + this.id }); },
  invoice() { wx.navigateTo({ url: '/pages/invoice/invoice?id=' + this.id }); },
  again() { wx.navigateTo({ url: '/pages/service/service?id=' + this.data.order.svcId }); },
  remove() {
    wx.showModal({
      title: '删除订单', content: '删除后不能恢复', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { db.removeOrder(this.id); wx.navigateBack(); } }
    });
  },
  copyId() { wx.setClipboardData({ data: this.data.order.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
