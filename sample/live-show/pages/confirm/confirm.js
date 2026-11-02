const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');
const payment = require('../../utils/pay.js');

Page({
  data: { show: null, ses: null, items: [], total: 0, viewers: [], need: 0, chosen: 0, phone: '', phoneErr: '', agree: false, submitting: false },
  onLoad() {
    const dr = store.draft();
    if (!dr) { wx.showToast({ title: '订单信息已失效', icon: 'none' }); setTimeout(() => wx.navigateBack(), 800); return; }
    const show = D.getShow(dr.showId);
    const ses = show.sessions.find((s) => s.id === dr.sessionId);
    this.draft = dr;
    this.setData({
      show, ses, items: dr.items, need: dr.items.length,
      total: dr.items.reduce((a, it) => a + it.price, 0),
      phone: wx.getStorageSync('ls_phone') || ''
    });
    store.setPick([]);
  },
  onShow() { this.loadViewers(); },
  loadViewers() {
    const pick = store.pick();
    const viewers = store.viewers().map((v) => ({ id: v.id, name: v.name, type: v.type, mask: v.mask, on: pick.indexOf(v.id) >= 0 }));
    this.setData({ viewers, chosen: viewers.filter((v) => v.on).length });
  },
  toggleViewer(e) {
    const id = e.currentTarget.dataset.id;
    let pick = store.pick();
    if (pick.indexOf(id) >= 0) pick = pick.filter((x) => x !== id);
    else {
      if (pick.length >= this.data.need) { wx.showToast({ title: '本单 ' + this.data.need + ' 张票，需选 ' + this.data.need + ' 位观演人', icon: 'none' }); return; }
      pick.push(id);
    }
    store.setPick(pick);
    this.loadViewers();
  },
  addViewer() { wx.navigateTo({ url: '/pages/viewer/viewer?pick=' + this.data.need }); },
  onPhone(e) { this.setData({ phone: e.detail.value, phoneErr: '' }); },
  checkPhone() {
    const ok = /^1[3-9]\d{9}$/.test(this.data.phone);
    this.setData({ phoneErr: ok || !this.data.phone ? '' : '手机号格式不正确' });
    return ok;
  },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  showNotice() {
    wx.showModal({
      title: '购票须知',
      content: D.NOTICE.map((n) => n.k + '：' + n.v).join('\n'),
      showCancel: false, confirmText: '我知道了'
    });
  },
  submit() {
    if (this.data.submitting) return;
    if (this.data.chosen !== this.data.need) { wx.showToast({ title: '请选择 ' + this.data.need + ' 位观演人（一证一票）', icon: 'none' }); return; }
    if (!this.checkPhone()) { wx.showToast({ title: this.data.phone ? '手机号格式不正确' : '请填写联系手机', icon: 'none' }); return; }
    if (!this.data.agree) { wx.showToast({ title: '请先阅读并同意购票须知', icon: 'none' }); return; }
    // 座位可能已被本机另一张待支付订单锁住
    const taken = store.takenSeats(this.data.ses.id);
    const clash = this.data.items.find((it) => it.seatId && taken[it.seatId]);
    if (clash) {
      wx.showModal({ title: '座位已被锁定', content: clash.label + ' 已在其他订单中，请重新选座', showCancel: false, success: () => wx.navigateBack() });
      return;
    }
    wx.setStorageSync('ls_phone', this.data.phone);
    this.setData({ submitting: true });
    // 热门场次模拟排队：排队 → 锁座 → 下单
    wx.showLoading({ title: '排队中，前方 128 人', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      wx.showLoading({ title: this.data.show.seat ? '正在锁定座位' : '正在锁定库存', mask: true });
      setTimeout(() => { wx.hideLoading(); this.createOrder(); }, 600);
    }, 900);
  },
  createOrder() {
    const s = this.data.show, ses = this.data.ses;
    const pick = store.pick();
    const viewers = store.viewers().filter((v) => pick.indexOf(v.id) >= 0).map((v) => ({ name: v.name, mask: v.mask, type: v.type }));
    const order = {
      id: F.orderNo(), showId: s.id, title: s.title, poster: s.poster, venue: s.venue, city: s.city,
      sessionId: ses.id, sessionText: ses.date + ' ' + ses.week + ' ' + ses.time,
      items: this.data.items, viewers, phone: this.data.phone, amount: this.data.total,
      status: 'unpaid', createdAt: Date.now(), expireAt: Date.now() + 15 * 60000, refundable: s.refundable
    };
    store.upsertOrder(order);
    store.setDraft(null);
    payment.pay(order, (ok, o) => {
      this.setData({ submitting: false });
      if (ok) wx.redirectTo({ url: '/pages/result/result?id=' + o.id });
      else wx.redirectTo({ url: '/pages/ticket/ticket?id=' + o.id });
    });
  }
});
