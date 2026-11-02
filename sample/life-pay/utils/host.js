// 要宿主才能做的事（支付、打电话、复制）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '');
}
// 付款：resolve(true) 付成功，resolve(false) 用户取消
function pay(amount, desc) {
  return new Promise(resolve => {
    const fallback = () => wx.showModal({
      title: '确认支付',
      content: desc + '\n实付 ¥' + (Math.round(amount * 100) / 100).toFixed(2),
      confirmText: '支付',
      confirmColor: '#14a06f',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    });
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'lp' + Date.now().toString(36),
      package: 'prepay_id=demo',
      signType: 'RSA',
      paySign: 'demo',
      total: Math.round(amount * 100),
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    });
  });
}
function call(phone) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: '服务热线', content: phone + '\n请用手机拨打', showCancel: false, confirmText: '知道了' });
    }
  });
}
function copy(text, title) {
  wx.setClipboardData({ data: text, success: () => { if (title) wx.showToast({ title, icon: 'none' }); } });
}
module.exports = { pay, call, copy, cancelled };
