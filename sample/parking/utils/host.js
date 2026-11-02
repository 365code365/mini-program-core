// 要宿主才能做的事（支付、打电话、导航、发票抬头）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '')
}
// 付款：resolve(true) 付成功，resolve(false) 用户取消。宿主没有支付能力时弹「确认支付」
function pay(amount, desc) {
  return new Promise(resolve => {
    const fallback = () => wx.showModal({
      title: '确认支付',
      content: desc + '，微信支付 ¥' + Number(amount).toFixed(2),
      confirmText: '支付',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    })
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'pk' + Date.now().toString(36),
      package: 'prepay_id=demo',
      signType: 'RSA',
      paySign: 'demo',
      total: Math.round(amount * 100),
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    })
  })
}
function call(phone) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: '联系电话', content: phone, showCancel: false, confirmText: '知道了' })
    }
  })
}
function distText(km) {
  return km < 1 ? Math.round(km * 1000) + 'm' : km.toFixed(1) + 'km'
}
// 导航去停车场：系统地图；宿主没有地图时弹地址与距离，可复制地址
function openMap(lot) {
  wx.openLocation({
    latitude: lot.lat,
    longitude: lot.lng,
    name: lot.name,
    address: lot.addr,
    scale: 17,
    fail: err => {
      if (cancelled(err)) return
      wx.showModal({
        title: lot.name,
        content: lot.addr + '\n距你 ' + distText(lot.dist) + '，驾车约 ' + Math.max(2, Math.round(lot.dist * 3)) + ' 分钟',
        confirmText: '复制地址',
        cancelText: '关闭',
        success: r => { if (r.confirm) copy(lot.addr) }
      })
    }
  })
}
// 导入微信发票抬头：resolve({ type, title, taxNo }) / resolve(null)
function invoiceTitle() {
  return new Promise(resolve => {
    wx.chooseInvoiceTitle({
      success: r => resolve({ type: r.type === 1 ? 'person' : 'company', title: r.title || '', taxNo: r.taxNumber || '' }),
      fail: err => {
        if (!cancelled(err)) wx.showToast({ title: '当前环境不支持导入，请手动填写', icon: 'none' })
        resolve(null)
      }
    })
  })
}
function copy(text) {
  wx.setClipboardData({ data: text })
}
module.exports = { pay, call, openMap, invoiceTitle, copy, distText }
