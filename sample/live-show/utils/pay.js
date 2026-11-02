// 支付：先调 wx.requestPayment；桌面引擎没有宿主支付会立刻 fail，
// 这时弹「模拟收银台」由用户再确认一次，确认才算付款成功，取消则订单保持待支付。
const store = require('./store.js');

function pay(order, cb) {
  wx.requestPayment({
    timeStamp: '' + Math.floor(Date.now() / 1000),
    nonceStr: order.id,
    package: 'prepay_id=' + order.id,
    signType: 'RSA',
    paySign: 'demo',
    success() { done(order, cb); },
    fail() {
      wx.showModal({
        title: '模拟收银台',
        content: '当前环境无法唤起微信支付。确认使用演示账户支付 ¥' + order.amount + '？',
        confirmText: '确认支付',
        cancelText: '稍后支付',
        success(r) {
          if (r.confirm) done(order, cb);
          else cb && cb(false, order);
        }
      });
    }
  });
}

function done(order, cb) {
  order.status = 'paid';
  order.paidAt = Date.now();
  store.upsertOrder(order);
  cb && cb(true, order);
}

module.exports = { pay };
