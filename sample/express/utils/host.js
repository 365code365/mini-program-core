// 要宿主才能做的事（支付、打电话、看地图、扫码、导入微信地址）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '')
}
// 付款：resolve(true) 付成功，resolve(false) 用户取消
function pay(amount, desc) {
  return new Promise(resolve => {
    const fallback = () => wx.showModal({
      title: '确认支付',
      content: desc + '\n微信支付 ¥' + Number(amount).toFixed(2),
      confirmText: '支付',
      confirmColor: '#2f6bff',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    })
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'ex' + Date.now().toString(36),
      package: 'prepay_id=demo',
      signType: 'RSA',
      paySign: 'demo',
      total: Math.round(amount * 100),
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    })
  })
}
function call(phone, title) {
  wx.makePhoneCall({
    phoneNumber: String(phone).replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: title || '联系电话', content: phone, showCancel: false, confirmText: '知道了' })
    }
  })
}
function copy(text, tip) {
  wx.setClipboardData({ data: text, success: () => { if (tip) wx.showToast({ title: tip, icon: 'none' }) } })
}
// 网点导航：系统地图；宿主没有地图时弹地址、距离与电话
function openMap(p) {
  wx.openLocation({
    latitude: p.lat,
    longitude: p.lng,
    name: p.name,
    address: p.addr,
    scale: 17,
    fail: err => {
      if (cancelled(err)) return
      wx.showModal({
        title: p.name,
        content: p.addr + '\n距你 ' + p.dist + '，电话 ' + p.phone,
        confirmText: '拨打电话',
        cancelText: '关闭',
        confirmColor: '#2f6bff',
        success: r => { if (r.confirm) call(p.phone, p.name) }
      })
    }
  })
}
// 扫面单条码：resolve(单号) / resolve('')（取消）/ resolve(null)（当前环境不能扫码，页面让用户手动输入）
function scan() {
  return new Promise(resolve => {
    wx.scanCode({
      scanType: ['barCode', 'qrCode'],
      success: r => resolve(String(r.result || '').replace(/[^0-9A-Za-z]/g, '').toUpperCase()),
      fail: err => resolve(cancelled(err) ? '' : null)
    })
  })
}
// 导入微信收货地址：resolve(地址) / resolve(null)（取消或当前环境不支持）
function chooseAddress() {
  return new Promise(resolve => {
    wx.chooseAddress({
      success: r => resolve({
        name: r.userName, phone: r.telNumber,
        regionArr: [r.provinceName, r.cityName, r.countyName],
        detail: r.detailInfoNew || r.detailInfo
      }),
      fail: err => {
        if (!cancelled(err)) wx.showToast({ title: '当前环境不支持导入，请手动填写', icon: 'none' })
        resolve(null)
      }
    })
  })
}
module.exports = { pay, call, copy, openMap, scan, chooseAddress }
