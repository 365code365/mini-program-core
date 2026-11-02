// 要宿主才能做的事（支付、打电话、导航、定位、扫码、选车牌、发票抬头）：
// 微信里、接了 hostApis 的宿主 App 里走系统能力；宿主没实现（桌面窗体、无头回归）时 fail 的 errMsg 不带 cancel，
// 就退回页面里的模拟交互，演示流程不断。用户自己点了取消（errMsg 带 cancel）则什么都不做。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '');
}
// 付款：resolve(true) 付成功，resolve(false) 用户取消
function pay(amount, desc, channel) {
  return new Promise(resolve => {
    const fallback = () => wx.showModal({
      title: '确认支付',
      content: `${desc}\n${channel || '微信支付'} ¥${amount}`,
      confirmText: '支付',
      confirmColor: '#00B87A',
      success: r => resolve(!!r.confirm),
      fail: () => resolve(false)
    });
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)),
      nonceStr: 'ev' + Date.now().toString(36),
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
      if (!cancelled(err)) wx.showModal({ title: '联系电话', content: phone, showCancel: false, confirmText: '知道了' });
    }
  });
}
// 导航到电站：系统地图（里面能选高德 / 腾讯 / 百度）；宿主没有地图时弹地址与距离，可复制地址
function openMap(s) {
  wx.openLocation({
    latitude: s.lat,
    longitude: s.lng,
    name: s.name,
    address: s.addr,
    scale: 16,
    fail: err => {
      if (cancelled(err)) return;
      wx.showModal({
        title: s.name,
        content: `${s.addr}\n距你 ${s.distText}，驾车约 ${Math.max(2, Math.round(s.dist * 3))} 分钟`,
        confirmText: '复制地址',
        cancelText: '关闭',
        success: r => { if (r.confirm) copy(s.addr); }
      });
    }
  });
}
// 定位：resolve(true) 拿到了系统定位；宿主不支持时 resolve(false)，页面按默认位置（杭州·西溪）处理
function locate() {
  return new Promise(resolve => {
    wx.getLocation({ type: 'gcj02', success: () => resolve(true), fail: () => resolve(false) });
  });
}
// 扫码：resolve(桩号) / resolve('')（取消）/ resolve(null)（当前环境不能扫码，页面让用户输入桩号）
// 桩上的码形如 https://ev.example.com/t/XX0128-02 或直接是桩号
function scan() {
  return new Promise(resolve => {
    wx.scanCode({
      onlyFromCamera: false,
      scanType: ['qrCode', 'barCode'],
      success: r => {
        const m = String(r.result || '').toUpperCase().match(/[A-Z]{2}\d{4}-\d{2}/);
        resolve(m ? m[0] : String(r.result || '').trim().toUpperCase());
      },
      fail: err => resolve(cancelled(err) ? '' : null)
    });
  });
}
// 选车牌（微信车牌键盘）：resolve('浙AD88520') / resolve(null)（取消或当前环境不支持，页面里手动填）
function plate() {
  return new Promise(resolve => {
    wx.chooseLicensePlate({
      success: r => resolve(r.plateNumber || null),
      fail: err => {
        if (!cancelled(err)) wx.showToast({ title: '当前环境不支持车牌键盘，请手动填写', icon: 'none' });
        resolve(null);
      }
    });
  });
}
// 导入微信发票抬头：resolve({ type, title, taxNo }) / resolve(null)
function invoiceTitle() {
  return new Promise(resolve => {
    wx.chooseInvoiceTitle({
      success: r => resolve({ type: r.type === 1 ? 'person' : 'company', title: r.title || '', taxNo: r.taxNumber || '' }),
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
module.exports = { pay, call, openMap, locate, scan, plate, invoiceTitle, copy };
