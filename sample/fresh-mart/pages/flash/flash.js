const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const pad = store.pad;
const REMIND = 'fm_flash_remind';
// 每 2 小时一场：当前场在抢，后面两场预告（同一批商品，预告场只能设提醒）
function slots(now) {
  const start = Math.floor(now.getHours() / 2) * 2;
  return [0, 1, 2].map(k => {
    const h = (start + k * 2) % 24;
    return { key: pad(h) + ':00', label: pad(h) + ':00', state: k === 0 ? '抢购中' : '即将开始', live: k === 0, nextDay: start + k * 2 >= 24 };
  });
}
Page({
  data: {
    slots: [],
    slotIdx: 0,
    hh: '00',
    mm: '00',
    ss: '00',
    list: [],
    cartCount: 0
  },
  onLoad() {
    this.setData({ slots: slots(new Date()) });
  },
  onShow() {
    this.load();
    this.tick();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.tick(), 1000);
  },
  onHide() {
    clearInterval(this.timer);
  },
  onUnload() {
    clearInterval(this.timer);
  },
  // 倒计时：在抢的场到本场结束，预告场到开抢
  tick() {
    const now = new Date();
    const s = this.data.slots[this.data.slotIdx];
    if (!s) return;
    const startHour = Math.floor(now.getHours() / 2) * 2 + this.data.slotIdx * 2;
    const target = new Date(now.getFullYear(), now.getMonth(), now.getDate(), startHour + (s.live ? 2 : 0), 0, 0).getTime();
    const left = Math.max(0, Math.floor((target - now.getTime()) / 1000));
    this.setData({ hh: pad(Math.floor(left / 3600)), mm: pad(Math.floor((left % 3600) / 60)), ss: pad(left % 60) });
  },
  load() {
    const live = this.data.slots[this.data.slotIdx] ? this.data.slots[this.data.slotIdx].live : true;
    const remind = wx.getStorageSync(REMIND) || {};
    const slot = (this.data.slots[this.data.slotIdx] || {}).key;
    const inCart = store.getCart();
    const list = data.flash.map(f => {
      const gd = data.findGoods(f.id);
      const out = live && f.stock >= 100;
      const taken = inCart.filter(x => x.id === f.id).reduce((n, x) => n + x.qty, 0);
      return {
        id: f.id, name: gd.name, spec: gd.spec, img: gd.img, price: gd.price, flashPrice: f.price,
        stock: live ? f.stock : 0, out, taken,
        reminded: !live && !!remind[slot + '|' + f.id],
        save: store.round2(gd.price - f.price)
      };
    });
    this.setData({ list, cartCount: store.cartCount() });
  },
  onSlot(e) {
    this.setData({ slotIdx: Number(e.currentTarget.dataset.idx) });
    this.load();
    this.tick();
  },
  onBuy(e) {
    const it = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    if (!it || it.out) return;
    const r = store.addToCart(it.id, 1);
    this.load();
    wx.showToast({ title: r.ok ? r.msg || `已抢到，购物车共 ${r.count} 件` : r.msg, icon: 'none' });
  },
  onRemind(e) {
    const slot = this.data.slots[this.data.slotIdx].key;
    const id = e.currentTarget.dataset.id;
    const remind = wx.getStorageSync(REMIND) || {};
    const k = slot + '|' + id;
    if (remind[k]) delete remind[k];
    else remind[k] = true;
    wx.setStorageSync(REMIND, remind);
    this.load();
    wx.showToast({ title: remind[k] ? `${slot} 开抢前 3 分钟提醒你` : '已取消提醒', icon: 'none' });
  },
  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },
  onCart() {
    wx.switchTab({ url: '/pages/cart/cart' });
  }
});
