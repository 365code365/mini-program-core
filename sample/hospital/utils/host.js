// 要宿主才能做的事（支付、打电话、看地图、定位）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
const data = require('./data.js');
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '');
}

// 付款：resolve(true) 付成功，resolve(false) 用户取消
function pay(amount, desc) {
  return new Promise(resolve => {
    const fallback = () => wx.showModal({
      title: '确认支付',
      content: desc + '\n微信支付 ¥' + amount,
      confirmText: '支付',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    });
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'hs' + Date.now().toString(36),
      package: 'prepay_id=demo',
      signType: 'RSA',
      paySign: 'demo',
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    });
  });
}

function call(phone, title) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/[\s-]/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: title || '联系电话', content: phone, showCancel: false, confirmText: '知道了' });
    }
  });
}

// 医院导航：系统地图；宿主没有地图能力时弹地址与交通指引
function openMap() {
  const h = data.hospital;
  wx.openLocation({
    latitude: h.lat,
    longitude: h.lng,
    name: h.name,
    address: h.address,
    scale: 17,
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: h.short, content: h.address + '\n' + h.traffic[0].k + '：' + h.traffic[0].v, showCancel: false, confirmText: '知道了' });
    }
  });
}

// 到院签到前的定位：resolve({ dist }) 米；宿主没有定位能力或用户拒绝授权时 resolve(null)，由页面改成手动确认
function locate() {
  return new Promise(resolve => {
    wx.getLocation({
      type: 'gcj02',
      success: r => {
        const h = data.hospital;
        const rad = Math.PI / 180;
        const dx = (r.longitude - h.lng) * rad * Math.cos(h.lat * rad) * 6371000;
        const dy = (r.latitude - h.lat) * rad * 6371000;
        resolve({ dist: Math.round(Math.sqrt(dx * dx + dy * dy)) });
      },
      fail: () => resolve(null)
    });
  });
}

function copy(text) {
  wx.setClipboardData({ data: text });
}

module.exports = { pay, call, openMap, locate, copy };
