// 康源大药房。启动时用 batchGetStorageSync 一次取回购物车 / 订单，清掉已下架的商品、推进订单状态，再 batchSetStorageSync 写回
const { BY_ID } = require('./utils/drugs.js');
const S = require('./utils/store.js');

App({
  globalData: { dept: '', buyNow: null },
  onLaunch() {
    const [cart, orders] = wx.batchGetStorageSync([S.K.cart, S.K.orders]);
    const kv = [];
    if (Array.isArray(cart)) {
      const clean = cart.filter(c => BY_ID[c.id]);
      if (clean.length !== cart.length) kv.push({ key: S.K.cart, value: clean });
    }
    if (Array.isArray(orders) && S.advanceOrders(orders)) kv.push({ key: S.K.orders, value: orders });
    if (kv.length) wx.batchSetStorageSync(kv);
  }
});
