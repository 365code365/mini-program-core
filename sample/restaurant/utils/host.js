// 要宿主才能做的事（支付、打电话、扫码）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '');
}

// 付款（金额单位：分）：resolve(true) 付成功，resolve(false) 用户取消
function pay(fen, desc) {
  const amount = Math.round(fen) / 100;
  return new Promise(resolve => {
    const fallback = () => wx.showModal({
      title: '确认支付',
      content: `${desc}\n微信支付 ¥${amount}`,
      confirmText: '支付',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    });
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'hw' + Date.now().toString(36),
      package: 'prepay_id=demo',
      signType: 'RSA',
      paySign: 'demo',
      total: Math.round(fen),
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    });
  });
}

function call(phone, title) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: title || '联系电话', content: phone, showCancel: false, confirmText: '知道了' });
    }
  });
}

// 扫码：resolve({ text }) 扫到了 / ({ cancel: true }) 用户取消 / ({ unsupported: true }) 当前宿主没有扫码能力
// 扫小程序码时内容在 path（带 ?table=…），扫普通二维码时在 result，两样都交给页面去认
function scan() {
  return new Promise(resolve => {
    wx.scanCode({
      onlyFromCamera: true,
      scanType: ['qrCode', 'wxCode'],
      success: r => resolve({ text: [r.path, r.result].filter(Boolean).join(' ') }),
      fail: err => resolve(cancelled(err) ? { cancel: true } : { unsupported: true })
    });
  });
}

module.exports = { pay, call, scan };
