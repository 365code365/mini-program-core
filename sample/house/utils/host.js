// 要宿主才能做的事（打电话、看地图、定位）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '')
}
function call(phone, who) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) {
        wx.showModal({
          title: who || '联系电话', content: phone, confirmText: '复制号码', confirmColor: '#ff5a1f',
          success: r => r.confirm && wx.setClipboardData({ data: phone.replace(/\s/g, '') })
        })
      }
    }
  })
}
// p: { lat, lng, name, addr, extra }
function openMap(p) {
  wx.openLocation({
    latitude: p.lat,
    longitude: p.lng,
    name: p.name,
    address: p.addr,
    scale: 16,
    fail: err => {
      if (!cancelled(err)) {
        wx.showModal({
          title: p.name, content: p.addr + (p.extra ? '\n' + p.extra : ''), confirmText: '复制地址', confirmColor: '#ff5a1f',
          success: r => r.confirm && wx.setClipboardData({ data: p.addr })
        })
      }
    }
  })
}
// 定位：resolve({ lat, lng }) / resolve(null)（取消或当前环境没有定位）
function locate() {
  return new Promise(resolve => {
    wx.getLocation({
      type: 'gcj02',
      success: r => resolve({ lat: r.latitude, lng: r.longitude }),
      fail: err => resolve(cancelled(err) ? null : { fallback: true })
    })
  })
}
function copy(text, tip) {
  wx.setClipboardData({ data: text, success: () => tip && wx.showToast({ title: tip, icon: 'none' }) })
}
module.exports = { call, openMap, locate, copy }
