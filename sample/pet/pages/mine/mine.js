const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 商城订单的四个状态入口（tab 与订单页一致）
const GOODS_TABS = [
  { key: 'unpaid', n: '待付款', ch: '付', bg: 'linear-gradient(135deg, #ffcf6b, #ffa53d)' },
  { key: 'paid', n: '待发货', ch: '发', bg: 'linear-gradient(135deg, #7ec8ff, #3b9cf0)' },
  { key: 'shipped', n: '待收货', ch: '收', bg: 'linear-gradient(135deg, #7fe0b0, #2fb57a)' },
  { key: 'rate', n: '待评价', ch: '评', bg: 'linear-gradient(135deg, #ff9fb5, #f2557a)' }
];
Page({
  data: {
    petCount: 0, bookings: [], bookedCount: 0, cartCount: 0, favCount: 0, addrCount: 0, points: 0, cardLeft: 0, remind: true,
    orderTabs: GOODS_TABS.map(t => Object.assign({ c: 0 }, t))
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    store.tick();
    const orders = store.getOrders();
    const goods = orders.filter(o => o.kind === 'goods');
    const booked = orders.filter(o => o.kind === 'service' && o.status === 'booked').sort((a, b) => store.startOf(a) - store.startOf(b));
    const count = { unpaid: 0, paid: 0, shipped: 0, rate: 0 };
    goods.forEach(o => {
      if (count[o.status] !== undefined) count[o.status] += 1;
      if (o.status === 'done' && !o.rating) count.rate += 1;
    });
    this.setData({
      petCount: store.getPets().length,
      bookedCount: booked.length,
      bookings: booked.slice(0, 3).map(o => {
        const p = o.date.split('-');
        return { id: o.id, dayNum: +p[2], monthText: (+p[1]) + '月', title: o.petName + ' · ' + o.svcNames, sub: o.week + ' ' + o.time + ' · ' + o.storeName, canCancel: store.canCancel(o) };
      }),
      cartCount: cart.count(),
      favCount: store.getFavs().length,
      addrCount: store.getAddrs().length,
      points: store.points(),
      cardLeft: store.cardLeft(),
      remind: wx.getStorageSync('pet_remind') !== false,
      orderTabs: GOODS_TABS.map(t => Object.assign({ c: count[t.key] }, t))
    });
  },
  cancel(e) {
    const o = store.findOrder(e.currentTarget.dataset.id);
    if (!store.canCancel(o)) {
      wx.showModal({ title: '无法在线取消', content: '服务开始前 2 小时内不能在线取消，如需改约请致电门店', showCancel: false, confirmText: '知道了' });
      return;
    }
    wx.showModal({
      title: '取消预约',
      content: '取消「' + o.petName + ' · ' + o.svcNames + '」？' + (o.pay ? '¥' + o.pay + ' 将原路退回' : '') + (o.cardUsed ? (o.pay ? '，' : '') + '洗护卡次数退回' : ''),
      confirmText: '取消预约',
      cancelText: '再想想',
      confirmColor: '#ff5b35',
      success: r => {
        if (!r.confirm) return;
        store.cancelOrder(o.id);
        wx.showToast({ title: '已取消预约', icon: 'none' });
        this.onShow();
      }
    });
  },
  onRemind(e) {
    wx.setStorageSync('pet_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    wx.showToast({ title: e.detail.value ? '疫苗、驱虫到期前 30 天会提醒你' : '已关闭到期提醒', icon: 'none' });
  },
  goPet() { wx.switchTab({ url: '/pages/pet/pet' }); },
  goCart() { wx.navigateTo({ url: '/pages/cart/cart' }); },
  goFavs() { wx.navigateTo({ url: '/pages/favs/favs' }); },
  goAddr() { wx.navigateTo({ url: '/pages/address/address' }); },
  goBooking() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  goServiceOrders() { wx.navigateTo({ url: '/pages/orders/orders?type=service' }); },
  goGoodsOrders(e) { wx.navigateTo({ url: '/pages/orders/orders?type=goods&tab=' + (e.currentTarget.dataset.key || 'all') }); },
  goDetail(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  // 会员资产：积分与洗护卡
  showVip() {
    wx.showModal({
      title: '萌宠 VIP',
      content: '积分 ' + this.data.points + '（实付 1 元得 1 分，下单立即到账）\n洗护次卡剩余 ' + this.data.cardLeft + ' 次（预约基础洗护时可抵扣）',
      showCancel: false,
      confirmText: '知道了'
    });
  },
  contact() {
    wx.showActionSheet({
      itemList: ['拨打客服 ' + data.SERVICE_PHONE + '（9:00-22:00）', '拨打 24h 宠物急诊 ' + data.EMERGENCY],
      success: r => host.call(r.tapIndex === 0 ? data.SERVICE_PHONE : data.EMERGENCY, r.tapIndex === 0 ? '萌宠家客服' : '24h 宠物急诊')
    });
  }
});
