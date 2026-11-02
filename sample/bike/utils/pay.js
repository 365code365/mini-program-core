// 微信支付：先走 wx.requestPayment，宿主不支持（桌面引擎默认立刻 fail）时弹模拟收银台，用户再确认一次才算付了。
// 用户主动取消（errMsg 带 cancel）不弹收银台。页面要有 data.cashier 与 onCashierConfirm / onCashierCancel（见 bindCashier）
function pay(page, opt, onOk) {
  const nonce = Math.random().toString(36).slice(2, 18);
  wx.requestPayment({
    timeStamp: String(Math.floor(Date.now() / 1000)),
    nonceStr: nonce,
    package: 'prepay_id=wx' + Date.now(),
    signType: 'RSA',
    paySign: 'mock-sign-' + nonce,
    success: () => onOk('wechat'),
    fail: (err) => {
      if (/cancel/.test((err && err.errMsg) || '')) {
        wx.showToast({ title: '已取消支付', icon: 'none' });
        return;
      }
      page._cashierOk = onOk;
      page.setData({ cashier: { show: true, amount: opt.amount, title: opt.title } });
    }
  });
}

// 收银台的两个回调：页面里写 onCashierConfirm() { pay.confirm(this) } / onCashierCancel() { pay.cancel(this) }
function confirm(page) {
  page.setData({ 'cashier.show': false });
  const ok = page._cashierOk;
  page._cashierOk = null;
  ok && ok('wechat');
}
function cancel(page) {
  page.setData({ 'cashier.show': false });
  page._cashierOk = null;
  wx.showToast({ title: '已取消支付', icon: 'none' });
}

module.exports = { pay, confirm, cancel };
