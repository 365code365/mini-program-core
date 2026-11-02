// 要宿主才能做的事（支付、打电话、看地图）：
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
      content: desc + '\n微信支付 ¥' + amount,
      confirmText: '支付',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    });
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'pet' + Date.now().toString(36),
      package: 'prepay_id=demo',
      signType: 'RSA',
      paySign: 'demo',
      total: Math.round(amount * 100),
      success: () => resolve(true),
      fail: err => (cancelled(err) ? resolve(false) : fallback())
    });
  });
}

function call(phone, title) {
  wx.makePhoneCall({
    phoneNumber: String(phone).replace(/[\s-]/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: title || '联系电话', content: String(phone), showCancel: false, confirmText: '知道了' });
    }
  });
}

// 门店导航：系统地图；宿主没有地图能力时弹地址
function openMap(s) {
  wx.openLocation({
    latitude: s.lat,
    longitude: s.lng,
    name: s.name,
    address: s.addr,
    scale: 17,
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: s.name, content: s.addr + '\n营业时间 ' + s.open, showCancel: false, confirmText: '知道了' });
    }
  });
}

// 导入微信收货地址：resolve(地址) / resolve(null)（取消或当前环境不支持）
function chooseAddress() {
  return new Promise(resolve => {
    wx.chooseAddress({
      success: r => resolve({
        name: r.userName,
        phone: r.telNumber,
        region: [r.provinceName, r.cityName, r.countyName],
        detail: r.detailInfoNew || r.detailInfo
      }),
      fail: err => {
        if (!cancelled(err)) wx.showToast({ title: '当前环境不支持导入，请手动填写', icon: 'none' });
        resolve(null);
      }
    });
  });
}

function copy(text) {
  wx.setClipboardData({ data: text });
}

module.exports = { pay, call, openMap, chooseAddress, copy };
