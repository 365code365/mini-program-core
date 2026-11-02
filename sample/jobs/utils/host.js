// 要宿主才能做的事（打电话、看地图、加日历）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '');
}
function call(phone, who) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) {
        wx.showModal({
          title: who || '联系电话', content: phone, confirmText: '复制号码', confirmColor: '#2E6BFF',
          success: r => r.confirm && wx.setClipboardData({ data: phone.replace(/\s/g, '') })
        });
      }
    }
  });
}
function openMap(p) {
  wx.openLocation({
    latitude: p.lat, longitude: p.lng, name: p.name, address: p.addr, scale: 16,
    fail: err => {
      if (!cancelled(err)) {
        wx.showModal({
          title: p.name, content: p.addr, confirmText: '复制地址', confirmColor: '#2E6BFF',
          success: r => r.confirm && wx.setClipboardData({ data: p.addr })
        });
      }
    }
  });
}
// 面试加进系统日历：resolve(true) 加上了（或用页内提醒代替）/ resolve(false) 用户取消
function addCalendar(ev) {
  return new Promise(resolve => {
    if (typeof wx.addPhoneCalendar !== 'function') return fallback();
    wx.addPhoneCalendar({
      title: ev.title, startTime: Math.floor(ev.ts / 1000), endTime: Math.floor(ev.ts / 1000) + 3600,
      description: ev.desc, location: ev.place, alarm: true, alarmOffset: 1800,
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    });
    function fallback() {
      wx.showModal({
        title: '设置面试提醒', content: ev.title + '\n' + ev.when + '\n将在开始前 30 分钟通过消息提醒你', confirmText: '设置', confirmColor: '#2E6BFF',
        success: r => resolve(!!r.confirm), fail: () => resolve(false)
      });
    }
  });
}
module.exports = { call, openMap, addCalendar };
