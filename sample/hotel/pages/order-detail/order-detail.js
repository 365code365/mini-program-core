const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];
Page({
  data: {
    o: null, h: null, room: null, st: null, lines: [], arrivals: data.ARRIVALS.map(a => a.t), arrivalIdx: 0, stay: '',
    starList: [1, 2, 3, 4, 5], score: 5, scoreText: STAR_TEXT[5], tags: [], rateText: ''
  },
  onLoad(q) {
    this.id = String(q.id || '');
    this.wantRate = String(q.rate) === '1';
    this.setData({ tags: data.RATE_TAGS.map(n => ({ n, on: false })) });
  },
  onShow() { this.load(); },
  onGoOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  load() {
    const o = store.findOrder(this.id);
    if (!o) { this.setData({ o: null }); return; }
    const h = data.hotelOf(o.hid);
    const room = h.rooms.find(r => r.id === o.rid) || h.rooms[0];
    const st = store.stageOf(o);
    const lines = [{ n: o.hourly ? '房费（' + o.count + '间 × 4小时）' : '房费（' + o.count + '间 × ' + o.nights + '晚）', v: '¥' + o.roomFee }];
    if (o.memberCut) lines.push({ n: '金卡会员 9.5 折', v: '-¥' + o.memberCut, cut: true });
    if (o.couponCut) lines.push({ n: o.couponTitle || '优惠券', v: '-¥' + o.couponCut, cut: true });
    if (o.pointsCut) lines.push({ n: '积分抵扣（' + o.pointsUsed + ' 积分）', v: '-¥' + o.pointsCut, cut: true });
    const stay = o.hourly
      ? store.short(o.checkin) + ' ' + store.week(o.checkin) + ' ' + o.start + ' 起 · 4 小时'
      : store.short(o.checkin) + ' ' + store.week(o.checkin) + ' - ' + store.short(o.checkout) + ' ' + store.week(o.checkout) + ' · ' + o.nights + '晚';
    this.setData({
      o, h, room, st, lines, stay, guestsText: o.guests.join('、'), refund: o.refund !== undefined ? o.refund : o.total,
      arrivalIdx: Math.max(0, data.ARRIVALS.findIndex(a => a.t === o.arrival))
    });
    if (this.wantRate && st.canRate) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 0 });
    }
  },
  onCancel() {
    const o = this.data.o;
    const st = this.data.st;
    const firstNight = o.hourly ? o.total : Math.min(o.total, Math.round(o.total / o.nights));
    const refund = st.free ? o.total : store.round2(o.total - firstNight);
    wx.showModal({
      title: '取消订单',
      content: st.free ? '当前可免费取消，¥' + o.total + ' 将原路退回，优惠券与积分一并退回。' : '已过免费取消时间，将扣除首晚房费 ¥' + firstNight + '，退款 ¥' + refund + '。',
      confirmText: '确认取消', cancelText: '再想想', confirmColor: '#FF5A36',
      success: r => {
        if (!r.confirm) return;
        const done = store.cancelOrder(o.id);
        wx.showToast({ title: done ? '已取消，退款 ¥' + done.refund + ' 原路退回' : '订单已无法取消', icon: 'none' });
        this.load();
      }
    });
  },
  onCall() { host.call(this.data.h.phone, this.data.h.name); },
  onNav() { host.openMap(this.data.h); },
  onRebook() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.data.o.hid }); },
  onArrival(e) {
    const first = data.firstArrival(this.data.o.checkin, store.ts, Date.now());
    if (Number(e.detail.value) < first) { wx.showToast({ title: '该时段已过，请选 ' + data.ARRIVALS[first].t, icon: 'none' }); return; }
    const a = data.ARRIVALS[Number(e.detail.value)];
    store.updateOrder(this.id, { arrival: a.t, arrivalStart: a.s });
    wx.showToast({ title: '已通知酒店：' + a.t + ' 到店', icon: 'none' });
    this.load();
  },
  onCopy() { host.copy(this.data.o.id); },
  onStar(e) {
    const score = Number(e.currentTarget.dataset.n);
    this.setData({ score, scoreText: STAR_TEXT[score] });
  },
  onTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ ['tags[' + i + '].on']: !this.data.tags[i].on });
  },
  onRateText(e) { this.setData({ rateText: e.detail.value }); },
  onSubmitRate() {
    const rating = { score: this.data.score, tags: this.data.tags.filter(t => t.on).map(t => t.n), text: this.data.rateText.trim(), time: store.dateTime(Date.now()) };
    store.updateOrder(this.id, { rating });
    store.changePoints(50, '点评奖励 ' + this.id);
    wx.showToast({ title: '感谢点评，+50 积分', icon: 'none' });
    this.load();
  }
});
