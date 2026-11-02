// 支付：先调 wx.requestPayment（真机由微信收银台完成），桌面引擎里它立刻 fail，
// 这时页面弹自己的「模拟收银台」（components/cashier），用户确认后才算付款成功。
const store = require('./store.js');

function request(order, onFallback) {
  const ts = String(Math.floor(Date.now() / 1000));
  wx.requestPayment({
    timeStamp: ts, nonceStr: 'ty' + ts, package: 'prepay_id=wx' + order.no, signType: 'RSA', paySign: 'mock-sign',
    success: () => { markPaid(order.id); wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + order.id }); },
    fail: (err) => {
      // 用户主动取消（真机）与环境不支持（桌面）都走这里；errMsg 带 cancel 的视为取消
      if (/cancel/.test(err && err.errMsg || '') && wx.canIUse('requestPayment')) { onFallback(false); return; }
      onFallback(true);
    }
  });
}

// 付款成功：拼团单进「待成团」并建团（参团满员立即成团），普通单进「待发货」
function markPaid(orderId) {
  const o = store.orderById(orderId);
  if (!o || o.status !== 'unpaid') return o;
  const now = Date.now();
  if (o.mode === 'group' || o.mode === 'join') {
    store.updateOrder(o.id, { status: 'grouping', paidAt: now });
    const join = o.joinFrom ? store.openGroups(o.items[0].goodsId).find((x) => x.id === o.joinFrom) || o.joinSnap : null;
    const grp = store.createGroup(store.orderById(o.id), join);
    store.updateOrder(o.id, { groupId: grp.id });
  } else {
    store.updateOrder(o.id, { status: 'paid', paidAt: now });
  }
  return store.orderById(o.id);
}

module.exports = { request, markPaid };
