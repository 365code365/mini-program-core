const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: {
    h: null, room: null, count: 1, minCount: 1, guests: [''], guestSeq: 0, phone: '', arrivals: data.ARRIVALS.map(a => a.t), arrival: 3,
    starts: [], startIdx: 0, hourly: false,
    invoice: false, invTypes: ['个人', '企业'], invType: 0, invTitle: '', taxNo: '', invSeq: 0, needs: '',
    p: {}, couponText: '', usePoints: false, points: 0
  },
  onLoad(q) {
    const h = data.hotelOf(q.hid);
    const room = h.rooms.find(r => r.id === q.rid) || h.rooms[0];
    const s = store.getSearch();
    const last = wx.getStorageSync(store.K.contact) || {};
    const hourly = !!room.hours;
    // 人数超过每间可住人数时按人数多订房间
    const minCount = hourly ? 1 : Math.ceil((s.adults + s.children) / room.max);
    let count = Math.max(hourly ? 1 : s.rooms, minCount);
    if (count > room.left) { count = room.left; wx.showToast({ title: '该房型仅剩 ' + room.left + ' 间', icon: 'none' }); }
    const guests = [last.name || ''];
    for (let i = 1; i < count; i++) guests.push('');
    // 钟点房：当天只能订还没过的开始时间
    const now = Date.now();
    const starts = data.HOUR_STARTS.filter(t => store.ts(s.checkin, t) > now + 30 * 60000);
    // 当天入住：已经过去的到店时段不能选（最后一档「次日 06:00 前」总能选），默认挑 18:00-20:00 或之后第一个还来得及的
    this.firstArrival = data.firstArrival(s.checkin, store.ts, now);
    this.setData({
      arrival: Math.max(3, this.firstArrival),
      h, room, hourly, count, minCount, guests, phone: last.phone || '', starts, startIdx: 0,
      checkin: s.checkin, checkout: s.checkout, inShort: store.short(s.checkin), inWeek: store.week(s.checkin),
      outShort: store.short(s.checkout), outWeek: store.week(s.checkout), nights: store.nights(s.checkin, s.checkout)
    });
    this.calc();
  },
  onShow() {
    // 常用入住人 / 发票抬头页选好的，回来时填进表单
    const names = wx.getStorageSync(store.K.guestPick);
    if (Array.isArray(names) && names.length) {
      wx.removeStorageSync(store.K.guestPick);
      const g = this.data.guests.slice();
      names.slice(0, g.length).forEach((n, i) => { g[i] = n; });
      this.setData({ guests: g, guestSeq: this.data.guestSeq + 1 });
    }
    const vid = wx.getStorageSync(store.K.invPick);
    if (vid) {
      wx.removeStorageSync(store.K.invPick);
      const v = store.getInvoices().find(x => x.id === vid);
      if (v) this.setData({ invoice: true, invType: v.type, invTitle: v.title, taxNo: v.taxNo || '', invSeq: this.data.invSeq + 1 });
    }
    this.setData({ points: store.getPoints() });
    this.calc();
  },
  calc() {
    const d = this.data;
    const p = store.price(d.h, d.room, d.count, d.nights, this.couponId, d.usePoints);
    let couponText = p.usable ? p.usable + ' 张可用' : '暂无可用';
    if (p.couponCut) couponText = '-¥' + p.couponCut;
    else if (this.couponId === '') couponText = '不使用优惠券';
    this.setData({ p, couponText, usePoints: d.usePoints && p.pointsMax > 0 });
  },
  onMinus() {
    if (this.data.count <= this.data.minCount) {
      if (this.data.minCount > 1) wx.showToast({ title: '按人数至少需要 ' + this.data.minCount + ' 间', icon: 'none' });
      return;
    }
    this.setData({ count: this.data.count - 1, guests: this.data.guests.slice(0, -1) });
    this.calc();
  },
  onPlus() {
    if (this.data.count >= this.data.room.left) { wx.showToast({ title: '该房型仅剩 ' + this.data.room.left + ' 间', icon: 'none' }); return; }
    this.setData({ count: this.data.count + 1, guests: this.data.guests.concat(['']) });
    this.calc();
  },
  onGuest(e) { const g = this.data.guests.slice(); g[Number(e.currentTarget.dataset.idx)] = e.detail.value; this.setData({ guests: g }); },
  onPickGuests() { wx.navigateTo({ url: '/pages/guests/guests?select=1&max=' + this.data.count }); },
  onPhone(e) { this.setData({ phone: e.detail.value }); },
  onArrival(e) {
    const i = Number(e.detail.value);
    if (i < this.firstArrival) {
      wx.showToast({ title: '该时段已过，请选 ' + data.ARRIVALS[this.firstArrival].t, icon: 'none' });
      this.setData({ arrival: this.firstArrival });
      return;
    }
    this.setData({ arrival: i });
  },
  onStart(e) { this.setData({ startIdx: Number(e.detail.value) }); },
  onInvoice(e) { this.setData({ invoice: e.detail.value }); },
  onInvType(e) { this.setData({ invType: Number(e.currentTarget.dataset.idx) }); },
  onInvTitle(e) { this.setData({ invTitle: e.detail.value }); },
  onTaxNo(e) { this.setData({ taxNo: e.detail.value }); },
  onPickInvoice() { wx.navigateTo({ url: '/pages/invoices/invoices?select=1' }); },
  // 特殊要求：点一项加上，再点一次去掉
  onNeeds() {
    wx.showActionSheet({
      itemList: data.NEEDS,
      success: r => {
        const cur = this.data.needs ? this.data.needs.split('、') : [];
        const n = data.NEEDS[r.tapIndex];
        const i = cur.indexOf(n);
        if (i >= 0) cur.splice(i, 1); else cur.push(n);
        this.setData({ needs: cur.join('、') });
      }
    });
  },
  onCoupon() {
    const d = this.data;
    const valid = store.validCoupons();
    const fee = d.p.roomFee;
    const items = valid.map(c => {
      const cut = store.couponCut(c, fee, d.h, d.room);
      return cut ? c.title + '  省 ¥' + cut : c.title + '（' + c.cond + '，不可用）';
    });
    items.push('不使用优惠券');
    wx.showActionSheet({
      itemList: items,
      success: r => {
        if (r.tapIndex === valid.length) this.couponId = '';
        else {
          const c = valid[r.tapIndex];
          if (!store.couponCut(c, fee, d.h, d.room)) { wx.showToast({ title: '当前订单不满足使用条件', icon: 'none' }); return; }
          this.couponId = c.id;
        }
        this.calc();
      }
    });
  },
  onPoints(e) { this.setData({ usePoints: e.detail.value }); this.calc(); },
  onSubmit() {
    if (this.paying) return;
    const d = this.data;
    const guests = d.guests.map(g => g.trim());
    let err = '';
    if (guests.some(g => !g)) err = '请填写每间房的住客姓名';
    else if (guests.some(g => g.length < 2)) err = '住客姓名至少 2 个字，与证件一致';
    else if (!/^1\d{10}$/.test(d.phone)) err = '请输入正确的手机号';
    else if (d.hourly && !d.starts.length) err = '今天的钟点房已过可订时间，请改订明天';
    else if (d.invoice && !d.invTitle.trim()) err = '请填写发票抬头';
    else if (d.invoice && d.invType === 1 && !/^[0-9A-Z]{15,20}$/.test(d.taxNo.trim().toUpperCase())) err = '请填写 15-20 位纳税人识别号';
    if (err) { wx.showToast({ title: err, icon: 'none' }); return; }
    this.paying = true;
    const when = d.hourly ? d.inShort + ' ' + d.starts[d.startIdx] + ' 起 4 小时' : d.inShort + '入住 ' + d.nights + ' 晚';
    host.pay(d.p.total, d.h.name + '\n' + d.room.name + ' × ' + d.count + ' 间，' + when).then(ok => {
      this.paying = false;
      if (ok) this.place(guests);
      else wx.showToast({ title: '已取消支付', icon: 'none' });
    });
  },
  place(guests) {
    const d = this.data;
    const p = d.p;
    const now = Date.now();
    const a = data.ARRIVALS[d.arrival];
    const start = d.hourly ? d.starts[d.startIdx] : '';
    const order = {
      id: 'HT' + now, hid: d.h.id, rid: d.room.id, hourly: d.hourly, hours: d.room.hours || 0, start, count: d.count,
      checkin: d.checkin, checkout: d.hourly ? d.checkin : d.checkout, nights: d.hourly ? 0 : d.nights,
      guests, phone: d.phone, arrival: d.hourly ? '' : a.t, arrivalStart: d.hourly ? start : a.s, needs: d.needs,
      invoice: d.invoice ? { type: d.invType, title: d.invTitle.trim(), taxNo: d.invType === 1 ? d.taxNo.trim().toUpperCase() : '' } : null,
      roomFee: p.roomFee, memberCut: p.memberCut, couponId: p.coupon ? p.coupon.id : '', couponTitle: p.coupon ? p.coupon.title : '', couponCut: p.couponCut,
      pointsUsed: p.pointsCut * 100, pointsCut: p.pointsCut, total: p.total, earn: Math.floor(p.total), createdAt: now, status: 'paid'
    };
    store.addOrder(order);
    if (order.couponId) store.useCoupon(order.couponId);
    if (order.pointsUsed) store.changePoints(-order.pointsUsed, '订房抵扣 ' + order.id);
    if (order.earn) store.changePoints(order.earn, '订房得积分 ' + order.id);
    wx.setStorageSync(store.K.contact, { name: guests[0], phone: d.phone });
    store.rememberGuests(guests);
    if (order.invoice && !store.getInvoices().some(v => v.title === order.invoice.title)) store.upsertInvoice(Object.assign({}, order.invoice));
    wx.showToast({ title: '预订成功', icon: 'success' });
    wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + order.id });
  }
});
