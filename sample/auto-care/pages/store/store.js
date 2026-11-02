const util = require('../../utils/util.js');
const { STORES, SERVICES, CATS } = require('../../utils/data.js');

const REVIEWS = {
  s1: [
    { user: '浙A·3J**8 车主', star: 5, text: '小保养 50 分钟就好了，技师把旧机滤拿给我看，透明。', date: '2026-10-21' },
    { user: '浙A·Q7**2 车主', star: 5, text: '钣喷颜色对得很准，看不出补过漆。', date: '2026-10-08' }
  ],
  s2: [{ user: '浙A·N2**6 车主', star: 5, text: '商场里停车方便，逛一圈回来车已经洗好了。', date: '2026-10-26' }],
  s3: [{ user: '浙A·K9**1 车主', star: 4, text: '轮胎规格很全，四轮定位后方向盘不跑偏了。', date: '2026-10-17' }],
  s4: [{ user: '浙A·H5**3 车主', star: 5, text: '年检代办很省心，下午就拿到了合格标志。', date: '2026-09-30' }]
};

Page({
  data: { store: null, groups: [], reviews: [], dist: '' },
  onLoad(q) {
    const store = STORES.find((s) => s.id === q.id) || STORES[0];
    const groups = CATS.map((c) => ({ id: c.id, name: c.name, items: SERVICES.filter((v) => v.cat === c.id) }));
    this.setData({ store, groups, reviews: REVIEWS[store.id] || [] });
    wx.setNavigationBarTitle({ title: store.short });
    util.getLocation((loc) => this.setData({ dist: util.distanceKm(loc, store) + ' km' }));
  },
  call() { util.callPhone(this.data.store.phone, this.data.store.name); },
  nav() { util.openStoreLocation(this.data.store); },
  copyAddr() { wx.setClipboardData({ data: this.data.store.address }); },
  pickService(e) {
    wx.navigateTo({ url: '/pages/service/service?storeId=' + this.data.store.id + '&sid=' + e.currentTarget.dataset.id });
  },
  book() { wx.navigateTo({ url: '/pages/service/service?storeId=' + this.data.store.id + '&cat=care' }); },
  onShareAppMessage() { return { title: this.data.store.name + ' · 在线预约保养洗车', path: '/pages/store/store?id=' + this.data.store.id }; }
});
