// 要宿主才能做的事（支付、打电话、看地图、定位、导入发票抬头、复制）：
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
      nonceStr: 'tq' + Date.now().toString(36),
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

// 看地图 / 导航：宿主没有地图能力时弹地址，可以复制给打车软件
function openMap(h) {
  wx.openLocation({
    latitude: h.lat,
    longitude: h.lng,
    name: h.name,
    address: h.city + h.addr,
    scale: 16,
    fail: err => {
      if (cancelled(err)) return;
      wx.showModal({
        title: h.name, content: h.city + h.addr + '\n' + h.dist, confirmText: '复制地址', cancelText: '关闭',
        success: r => { if (r.confirm) copy(h.city + h.addr); }
      });
    }
  });
}

// 定位：resolve({ lat, lng }) / resolve(null)（宿主不支持或用户拒绝）
function locate() {
  return new Promise(resolve => {
    wx.getLocation({ type: 'gcj02', success: r => resolve({ lat: r.latitude, lng: r.longitude }), fail: () => resolve(null) });
  });
}

// 导入微信发票抬头：resolve({ type, title, taxNo }) / resolve(null)
function invoiceTitle() {
  return new Promise(resolve => {
    wx.chooseInvoiceTitle({
      success: r => resolve({ type: r.type === 1 ? 0 : 1, title: r.title || '', taxNo: r.taxNumber || '' }),
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

module.exports = { pay, call, openMap, locate, invoiceTitle, copy };
