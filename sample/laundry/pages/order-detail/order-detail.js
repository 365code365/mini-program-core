const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const push = require('../../utils/push.js');

const LAST_AUTO = 7; // 推送最多推到「配送中」，「已签收」要用户确认

function hm(ts) {
  const d = new Date(ts);
  return store.pad(d.getMonth() + 1) + '-' + store.pad(d.getDate()) + ' ' + store.pad(d.getHours()) + ':' + store.pad(d.getMinutes());
}

Page({
  data: { o: {}, statusName: '', stepDesc: '', nodes: [], timeline: [], showQc: false, qc: data.QC, pickText: '', createdText: '', pushMode: '' },

  onLoad(q) {
    this.id = q.id;
    if (!store.order(this.id)) {
      wx.showToast({ title: '订单不存在', icon: 'none' });
      setTimeout(() => wx.navigateBack(), 800);
      return;
    }
    this.render();
  },

  // 页面可见时才连推送；离开（隐藏 / 卸载）就断开
  onShow() {
    this.render();
    const o = store.order(this.id);
    if (o && !o.cancelled && o.step < LAST_AUTO && !this.conn) {
      this.conn = push.connect(this.id, msg => this.onPush(msg), mode => this.setData({ pushMode: mode }));
    }
  },

  onHide() { this.disconnect(); },
  onUnload() { this.disconnect(); },

  disconnect() {
    if (this.conn) { this.conn.close(); this.conn = null; }
  },

  onPush(msg) {
    const o = store.order(this.id);
    if (!o || o.cancelled) return this.disconnect();
    let next = o.step;
    if (msg.type === 'step' && typeof msg.step === 'number') next = Math.max(o.step, Math.min(msg.step, LAST_AUTO));
    else if (msg.type === 'advance') next = Math.min(o.step + 1, LAST_AUTO);
    if (next === o.step) return;
    this.setStep(o, next);
    if (next >= LAST_AUTO) this.disconnect();
    if (next === 2) wx.vibrateShort({ type: 'light', fail() {} });
  },

  setStep(o, step) {
    for (let s = o.step + 1; s <= step; s++) o.stepAt[s] = Date.now();
    o.step = step;
    store.saveOrder(o);
    this.render();
  },

  render() {
    const o = store.order(this.id);
    if (!o) return;
    const main = store.mainStatus(o);
    const timeline = [];
    for (let s = o.step; s >= 0; s--) {
      timeline.push({ text: data.STEPS[s].text, desc: data.STEPS[s].desc, time: hm(o.stepAt[s] || o.createdAt) });
    }
    if (o.cancelled) timeline.unshift({ text: '订单已取消', desc: o.cancelReason || '用户取消', time: hm(o.cancelledAt) });
    this.setData({
      o, statusName: store.statusName(o), stepDesc: data.STEPS[o.step].desc,
      nodes: data.STATUS.map(s => ({ name: s.name, on: main >= s.key })),
      timeline, showQc: !o.cancelled && o.step >= 3,
      pickText: o.pickup.label.split(' ').slice(0, 2).join(' ') + ' ' + o.pickup.slot,
      createdText: store.timeStr(o.createdAt)
    });
  },

  previewQc(e) {
    const urls = data.QC.map(x => x.src);
    wx.previewImage({ current: urls[e.currentTarget.dataset.i], urls });
  },

  copyNo() {
    wx.setClipboardData({ data: this.data.o.id, success: () => wx.showToast({ title: '订单号已复制', icon: 'none' }) });
  },

  contact() {
    wx.showActionSheet({
      itemList: ['拨打取件员电话', '复制订单号', '在线客服'],
      success: res => {
        if (res.tapIndex === 0) {
          wx.makePhoneCall({
            phoneNumber: data.COURIER.phone,
            fail: () => wx.showModal({ title: '取件员 ' + data.COURIER.name, content: '当前设备无法直接拨号，请手动拨打 ' + data.COURIER.phone + ' 转 ' + data.COURIER.no, showCancel: false })
          });
        } else if (res.tapIndex === 1) this.copyNo();
        else wx.showModal({ title: '在线客服', content: '客服在线时间 8:00-22:00。也可拨打 ' + data.SERVICE_PHONE + '，说明订单号 ' + this.data.o.id + ' 可更快处理。', showCancel: false });
      }
    });
  },

  cancel() {
    wx.showActionSheet({
      itemList: ['时间不合适', '衣物选错了', '不想洗了', '其他原因'],
      success: r => {
        const reason = ['时间不合适', '衣物选错了', '不想洗了', '其他原因'][r.tapIndex];
        wx.showModal({
          title: '确认取消订单？',
          content: '取件员上门前可免费取消，实付 ¥' + this.data.o.payable + ' 将在 1-3 个工作日原路退回。',
          confirmText: '取消订单',
          cancelText: '再想想',
          success: res => {
            if (!res.confirm) return;
            const o = store.order(this.id);
            if (o.step >= 2) return wx.showToast({ title: '已取件，无法取消', icon: 'none' });
            o.cancelled = true;
            o.cancelReason = reason;
            o.cancelledAt = Date.now();
            store.saveOrder(o);
            if (o.coupon) store.useCoupon(o.coupon.id, false);
            this.disconnect();
            this.render();
            wx.showToast({ title: '订单已取消', icon: 'none' });
          }
        });
      }
    });
  },

  confirmReceive() {
    wx.showModal({
      title: '确认已收到衣物？',
      content: '请检查衣物数量与洗护效果。签收后 72 小时内仍可申请免费返洗。',
      success: res => {
        if (!res.confirm) return;
        this.setStep(store.order(this.id), 8);
        wx.showToast({ title: '已签收', icon: 'success' });
      }
    });
  },

  goReview() { wx.navigateTo({ url: '/pages/review/review?id=' + this.id }); },

  // 再来一单：把衣物放回购物篮，去价目表
  again() {
    this.data.o.lines.forEach(l => store.setQty(l.id, l.qty));
    wx.switchTab({ url: '/pages/price/price' });
  }
});
