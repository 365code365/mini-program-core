// 要宿主才能做的事：微信里、接了 hostApis 的宿主 App 里走系统能力；
// 宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，就退回页面里的处理，流程不断。
// 用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '')
}
// 复制到剪贴板：resolve(true) 复制成功
function copy(text, okTitle) {
  return new Promise(resolve => {
    wx.setClipboardData({
      data: text,
      success: () => { if (okTitle) wx.showToast({ title: okTitle, icon: 'none' }); resolve(true) },
      fail: () => { wx.showModal({ title: '复制失败', content: '请长按预览区手动复制', showCancel: false }); resolve(false) }
    })
  })
}
// 订阅提醒（微信的订阅消息）：resolve(true) 已开启（宿主没有订阅能力时只在本机记下设置），resolve(false) 用户拒绝 / 取消
function subscribe(tmplId) {
  return new Promise(resolve => {
    if (typeof wx.requestSubscribeMessage !== 'function') { resolve(true); return }
    wx.requestSubscribeMessage({
      tmplIds: [tmplId],
      success: r => resolve(r[tmplId] !== 'reject'),
      fail: err => resolve(!cancelled(err))
    })
  })
}
module.exports = { cancelled, copy, subscribe }
