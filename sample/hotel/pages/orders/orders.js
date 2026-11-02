const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: {
    tabs: [{ key: 'all', name: '全部' }, { key: 'wait', name: '待入住' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }],
    tab: 'all', list: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const want = wx.getStorageSync(store.K.orderTab);
    if (want) { wx.removeStorageSync(store.K.orderTab); this.setData({ tab: want }); }
    this.load();
  },
  load() {
    const tab = this.data.tab;
    const list = store.getOrders().map(o => {
      const h = data.hotelOf(o.hid);
      const room = h.rooms.find(r => r.id === o.rid) || h.rooms[0];
      const st = store.stageOf(o);
      return {
        id: o.id, hid: o.hid, hotel: h.name, img: h.img, room: room.name, count: o.count, total: o.total, guests: o.guests.join('、'),
        when: o.hourly ? store.short(o.checkin) + ' ' + o.start + ' 起 · 4小时' : store.short(o.checkin) + ' - ' + store.short(o.checkout) + ' · ' + o.nights + '晚',
        key: st.key, text: st.text, canCancel: st.canCancel, canRate: st.canRate, type: o.hourly ? '钟点房' : h.type
      };
    }).filter(o => tab === 'all' || o.key === tab || (tab === 'wait' && o.key === 'in'));
    this.setData({ list });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.load(); },
  onOpen(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  onRate(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?rate=1&id=' + e.currentTarget.dataset.id }); },
  onCancel(e) {
    const id = String(e.currentTarget.dataset.id);
    const o = store.findOrder(id);
    const st = store.stageOf(o);
    const firstNight = o.hourly ? o.total : Math.min(o.total, Math.round(o.total / o.nights));
    wx.showModal({
      title: '取消订单',
      content: st.free ? '当前可免费取消，¥' + o.total + ' 将在 1-3 个工作日原路退回。' : '已过免费取消时间，将扣除首晚房费 ¥' + firstNight + '。',
      confirmText: '确认取消', confirmColor: '#FF5A36',
      success: r => {
        if (!r.confirm) return;
        const done = store.cancelOrder(id);
        this.load();
        wx.showToast({ title: done ? '已取消，退款 ¥' + done.refund : '订单已无法取消', icon: 'none' });
      }
    });
  },
  onContact(e) {
    const h = data.hotelOf(String(e.currentTarget.dataset.hid));
    host.call(h.phone, h.name);
  },
  onRebook(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.hid }); },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
