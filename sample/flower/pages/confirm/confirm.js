const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const PREFS = 'fl_sender_prefs';
Page({
  data: {
    lines: [], from: 'cart', addr: null, slots: data.slots, date: '', slotIdx: 2, minDate: '', maxDate: '',
    sender: '林先生', anonymous: false, callFirst: true, p: {}, couponText: '', saved: 0,
    cardIdx: -1, cardText: '', cardSeq: 0,
    cardTpls: ['生日快乐，岁岁平安！', '谢谢你一直在我身边。', '祝您早日康复。', '开业大吉，生意兴隆！']
  },
  onLoad(q) {
    // 直接打开（没有结算单）时，用购物车里勾选的商品
    let src = wx.getStorageSync('fl_checkout');
    if (!Array.isArray(src) || !src.length) src = store.getCart().filter(x => x.checked);
    this.src = src.map(x => Object.assign({}, x));
    // 一单只送一次：送达时间取第一束花选的，在这里可以统一改
    const first = this.src[0] || { date: store.dayAfter(1), slot: data.slots[2] };
    const fx = store.fixSlot(first.date, first.slot);
    const prefs = wx.getStorageSync(PREFS) || {};
    this.setData({
      from: q.from || 'cart', date: fx.date, slotIdx: Math.max(0, data.slots.indexOf(fx.slot)),
      minDate: store.dayAfter(0), maxDate: store.dayAfter(30),
      sender: prefs.sender || '林先生', anonymous: !!prefs.anonymous, callFirst: prefs.callFirst !== false
    });
  },
  onShow() {
    const a = store.currentAddr();
    this.setData({ addr: a ? Object.assign({}, a, { text: store.addrText(a), phoneMask: store.maskPhone(a.phone) }) : null });
    this.calc();
  },
  calc() {
    const lines = this.src.map(store.expand);
    const p = store.price(lines, this.couponId);
    let couponText = p.usable ? p.usable + ' 张可用' : '暂无可用';
    if (p.couponCut) couponText = '-¥' + p.couponCut;
    else if (this.couponId === '') couponText = '不使用优惠券';
    this.setData({ lines, p, couponText, saved: store.round2(p.activityCut + p.memberCut + p.couponCut) });
  },
  onAddr() { wx.navigateTo({ url: '/pages/address/address?select=1' }); },
  onDate(e) {
    const fx = store.fixSlot(e.detail.value, data.slots[this.data.slotIdx]);
    this.setData({ date: fx.date, slotIdx: data.slots.indexOf(fx.slot) });
    if (fx.changed) wx.showToast({ title: '所选时段来不及配送，已改为 ' + fx.date.slice(5) + ' ' + fx.slot, icon: 'none' });
  },
  onSlot(e) {
    const i = Number(e.detail.value);
    if (!store.slotOpen(this.data.date, data.slots[i])) {
      wx.showToast({ title: '这个时段来不及配送了，请选更晚的时段', icon: 'none' });
      return;
    }
    this.setData({ slotIdx: i });
  },
  onCoupon() {
    const valid = store.validCoupons();
    const p = this.data.p;
    const items = valid.map(c => {
      const cut = store.couponCut(c, this.data.lines, p);
      return cut ? c.title + '  省 ¥' + cut : c.title + '（' + c.cond + '，未满足）';
    });
    items.push('不使用优惠券');
    wx.showActionSheet({
      itemList: items,
      success: r => {
        if (r.tapIndex === valid.length) this.couponId = '';
        else {
          const c = valid[r.tapIndex];
          if (!store.couponCut(c, this.data.lines, p)) { wx.showToast({ title: '当前订单不满足使用条件', icon: 'none' }); return; }
          this.couponId = c.id;
        }
        this.calc();
      }
    });
  },
  onSender(e) { this.setData({ sender: e.detail.value }); },
  onAnon(e) {
    this.setData({ anonymous: e.detail.value });
    if (e.detail.value) wx.showToast({ title: '贺卡与配送员都不会透露你的名字', icon: 'none' });
  },
  onCall(e) { this.setData({ callFirst: e.detail.value }); },
  // 贺卡：每束花一张，结算前还能改
  editCard(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ cardIdx: i, cardText: this.src[i].card || '', cardSeq: this.data.cardSeq + 1 });
  },
  onCardInput(e) { this.setData({ cardText: e.detail.value }); },
  useTpl(e) { this.setData({ cardText: e.currentTarget.dataset.t, cardSeq: this.data.cardSeq + 1 }); },
  closeCard() { this.setData({ cardIdx: -1 }); },
  noop() {},
  saveCard() {
    const i = this.data.cardIdx;
    const text = this.data.cardText.trim().slice(0, 60);
    this.src[i].card = text;
    if (this.data.from === 'cart') {
      const cart = store.getCart();
      const hit = cart.find(x => x.key === this.src[i].key);
      if (hit) { hit.card = text; store.saveCart(cart); }
    }
    this.setData({ cardIdx: -1 });
    this.calc();
  },
  submit() {
    if (this.paying) return;
    const d = this.data;
    let err = '';
    if (!d.lines.length) err = '没有要结算的花束';
    else if (!d.addr) err = '请选择收花地址';
    else if (!d.anonymous && !d.sender.trim()) err = '请填写贺卡署名，或打开匿名送花';
    else if (!store.slotOpen(d.date, d.slots[d.slotIdx])) err = '所选时段来不及配送，请重新选择';
    if (err) { wx.showToast({ title: err, icon: 'none' }); return; }
    const count = d.lines.reduce((s, x) => s + x.qty, 0);
    this.paying = true;
    host.pay(d.p.total, '花语鲜花 · ' + count + ' 束花，' + d.date.slice(5) + ' ' + d.slots[d.slotIdx] + ' 送达').then(ok => {
      this.paying = false;
      if (ok) this.place();
      else wx.showToast({ title: '已取消支付', icon: 'none' });
    });
  },
  place() {
    const d = this.data;
    const p = d.p;
    const now = Date.now();
    const order = {
      no: 'HY' + store.dateTime(now).replace(/[-: ]/g, '') + store.pad(new Date(now).getSeconds()) + String(now).slice(-2),
      status: 'paid', createdAt: now, time: store.dateTime(now), date: d.date, slot: d.slots[d.slotIdx],
      receiver: d.addr.name, phone: d.addr.phoneMask, addr: d.addr.text,
      anonymous: d.anonymous, sender: d.anonymous ? '' : d.sender.trim(), callFirst: d.callFirst,
      goods: p.goods, activityCut: p.activityCut, memberCut: p.memberCut, memberName: p.memberName + ' ' + p.memberRate,
      couponId: p.coupon ? p.coupon.id : '', couponTitle: p.coupon ? p.coupon.title : '', couponCut: p.couponCut,
      total: p.total, payBy: 'wechat',
      items: d.lines.map(x => ({ id: x.id, stems: x.stems, pack: x.pack, qty: x.qty, card: x.card || '', unit: x.unit }))
    };
    store.addOrder(order);
    if (order.couponId) store.useCoupon(order.couponId);
    if (d.from === 'cart') store.saveCart(store.getCart().filter(x => !x.checked));
    wx.setStorageSync(PREFS, { sender: d.sender.trim(), anonymous: d.anonymous, callFirst: d.callFirst });
    wx.removeStorageSync('fl_checkout');
    wx.showToast({ title: '支付成功', icon: 'success' });
    wx.redirectTo({ url: '/pages/order-detail/order-detail?no=' + order.no });
  }
});
