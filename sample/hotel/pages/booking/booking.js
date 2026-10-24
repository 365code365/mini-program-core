const app = getApp();
const u = app.util;
const ARRIVALS = ['14:00 前', '14:00-16:00', '16:00-18:00', '18:00-20:00', '20:00-22:00', '22:00-24:00', '次日 06:00 前'];
const NEEDS = ['高楼层', '无烟房', '安静房', '加床（另付费）', '婴儿床'];

Page({
  data: {
    h: null, room: null, count: 1, guests: [''], phone: '', arrivals: ARRIVALS, arrival: 3,
    invoice: false, invTypes: ['个人', '企业'], invType: 0, invTitle: '', taxNo: '', needs: ''
  },
  onLoad(q) {
    const h = app.getHotel(q.hid);
    const room = h.rooms.find(r => r.id === q.rid) || h.rooms[0];
    const s = app.getSearch();
    const last = wx.getStorageSync('hotel_guest') || {};
    this.setData({
      h, room, guests: [last.name || ''], phone: last.phone || '',
      inShort: u.short(s.checkin), inWeek: u.week(s.checkin), outShort: u.short(s.checkout), outWeek: u.week(s.checkout),
      nights: u.nights(s.checkin, s.checkout), checkin: s.checkin, checkout: s.checkout
    });
    this.calc();
  },
  calc() {
    const roomFee = this.data.room.price * this.data.count * this.data.nights;
    const discount = Math.round(roomFee * 0.05);
    this.setData({ roomFee, discount, total: roomFee - discount });
  },
  onMinus() {
    if (this.data.count <= 1) return;
    this.setData({ count: this.data.count - 1, guests: this.data.guests.slice(0, -1) });
    this.calc();
  },
  onPlus() {
    if (this.data.count >= this.data.room.left) { wx.showToast({ title: '该房型仅剩 ' + this.data.room.left + ' 间', icon: 'none' }); return; }
    this.setData({ count: this.data.count + 1, guests: this.data.guests.concat(['']) });
    this.calc();
  },
  onGuest(e) { const g = this.data.guests.slice(); g[e.currentTarget.dataset.idx] = e.detail.value; this.setData({ guests: g }); },
  onPhone(e) { this.setData({ phone: e.detail.value }); },
  onArrival(e) { this.setData({ arrival: Number(e.detail.value) }); },
  onInvoice(e) { this.setData({ invoice: e.detail.value }); },
  onInvType(e) { this.setData({ invType: e.currentTarget.dataset.idx }); },
  onInvTitle(e) { this.setData({ invTitle: e.detail.value }); },
  onTaxNo(e) { this.setData({ taxNo: e.detail.value }); },
  onNeeds() {
    wx.showActionSheet({ itemList: NEEDS, success: (r) => {
      const cur = this.data.needs ? this.data.needs.split('、') : [];
      const n = NEEDS[r.tapIndex];
      if (cur.indexOf(n) < 0) cur.push(n);
      this.setData({ needs: cur.join('、') });
    } });
  },
  onSubmit() {
    const d = this.data;
    if (d.guests.some(g => !g.trim())) { wx.showToast({ title: '请填写住客姓名', icon: 'none' }); return; }
    if (!/^1\d{10}$/.test(d.phone)) { wx.showToast({ title: '请输入正确的手机号', icon: 'none' }); return; }
    if (d.invoice && !d.invTitle.trim()) { wx.showToast({ title: '请填写发票抬头', icon: 'none' }); return; }
    wx.showModal({
      title: '确认支付 ¥' + d.total,
      content: d.h.name + '\n' + d.room.name + ' × ' + d.count + '间，' + d.inShort + '入住',
      confirmText: '确认支付',
      success: (r) => {
        if (!r.confirm) return;
        wx.setStorageSync('hotel_guest', { name: d.guests[0], phone: d.phone });
        const orders = app.getOrders();
        orders.unshift({
          id: 'HT' + Date.now(), hid: d.h.id, hotel: d.h.name, img: d.h.img, room: d.room.name, count: d.count,
          checkin: d.checkin, checkout: d.checkout, inShort: d.inShort, outShort: d.outShort, nights: d.nights,
          guests: d.guests.join('、'), total: d.total, status: 'wait', arrival: d.arrivals[d.arrival], invoice: d.invoice
        });
        app.saveOrders(orders);
        wx.showToast({ title: '预订成功', icon: 'success' });
        setTimeout(() => wx.switchTab({ url: '/pages/orders/orders' }), 800);
      }
    });
  }
});
