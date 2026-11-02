// 微信支付：桌面引擎没有宿主支付会立刻 fail，弹「模拟收银台」让用户再确认，确认才算付款成功。
function wechatPay(amount, desc, cb) {
  wx.requestPayment({
    timeStamp: '' + Math.floor(Date.now() / 1000), nonceStr: 'yc' + Date.now(), package: 'prepay_id=demo', signType: 'RSA', paySign: 'demo',
    success() { cb(true); },
    fail() {
      wx.showModal({
        title: '模拟收银台', content: '当前环境无法唤起微信支付。确认使用演示账户支付 ¥' + amount + '（' + desc + '）？',
        confirmText: '确认支付', cancelText: '取消',
        success(r) { cb(!!r.confirm); }
      });
    }
  });
}
module.exports = { wechatPay };
