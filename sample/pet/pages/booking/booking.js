const data = require('../../utils/data.js');
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
const STAY = { id: 'stay', name: '寄养 · 独立房（1 晚）', desc: '每日 2 次视频回传 · 含猫砂/尿垫', price: 89, min: 0 };
const pad = v => (v < 10 ? '0' + v : '' + v);
const fmt = d => d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());

Page({
  data: {
    pets: data.pets, petId: 'p1', svcs: [], storeNames: data.stores.map(s => s.name), storeIdx: 0,
    today: '', maxDate: '', date: '', week: '', time: '14:00', pickup: false, remark: '', total: 0, totalMin: 0
  },
  onLoad(q) {
    const list = data.services.concat(STAY).map(s => Object.assign({}, s, { on: s.id === (q.svc || 's1') }));
    const now = new Date();
    const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    const max = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 30);
    const si = Math.max(0, data.stores.findIndex(s => s.id === q.store));
    this.setData({
      svcs: list, storeIdx: si, today: fmt(now), maxDate: fmt(max), date: fmt(tomorrow), week: WEEK[tomorrow.getDay()],
      petId: wx.getStorageSync('pet_current') || 'p1'
    });
    this.calc();
  },
  pickPet(e) { this.setData({ petId: e.currentTarget.dataset.id }); },
  toggleSvc(e) {
    const i = e.currentTarget.dataset.index;
    this.setData({ ['svcs[' + i + '].on']: !this.data.svcs[i].on });
    this.calc();
  },
  calc() {
    const on = this.data.svcs.filter(s => s.on);
    this.setData({
      total: on.reduce((a, s) => a + s.price, 0) + (this.data.pickup ? 20 : 0),
      totalMin: on.reduce((a, s) => a + s.min, 0)
    });
  },
  onStore(e) { this.setData({ storeIdx: Number(e.detail.value) }); },
  onDate(e) {
    const p = e.detail.value.split('-').map(Number);
    this.setData({ date: e.detail.value, week: WEEK[new Date(p[0], p[1] - 1, p[2]).getDay()] });
  },
  onTime(e) { this.setData({ time: e.detail.value }); },
  onPickup(e) { this.setData({ pickup: e.detail.value }); this.calc(); },
  onRemark(e) { this.setData({ remark: e.detail.value }); },
  submit() {
    const on = this.data.svcs.filter(s => s.on);
    if (!on.length) return wx.showToast({ title: '请至少选择一项服务', icon: 'none' });
    const pet = data.findPet(this.data.petId);
    const store = data.stores[this.data.storeIdx];
    const item = {
      id: 'B' + Date.now(), pet: pet.name, svc: on.map(s => s.name).join(' + '), store: store.name,
      date: this.data.date, week: this.data.week, time: this.data.time, total: this.data.total
    };
    const list = wx.getStorageSync('pet_bookings') || [];
    list.unshift(item);
    wx.setStorageSync('pet_bookings', list);
    wx.showModal({
      title: '预约成功',
      content: pet.name + ' · ' + item.svc + '\n' + item.date + ' ' + item.week + ' ' + item.time + ' 到店\n' + store.name,
      showCancel: false, confirmText: '好的',
      success: () => wx.switchTab({ url: '/pages/mine/mine' })
    });
  }
});
