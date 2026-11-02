// 要宿主才能做的事（支付、打电话、发短信、定位、导入发票抬头）：
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
      nonceStr: 'dd' + Date.now().toString(36),
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
    phoneNumber: String(phone).replace(/\s/g, ''),
    fail: err => {
      if (!cancelled(err)) wx.showModal({ title: title || '联系电话', content: String(phone), showCancel: false, confirmText: '知道了' });
    }
  });
}

// 发短信（分享行程给紧急联系人）：resolve(true) 发出去了
function sms(phones, names, content) {
  return new Promise(resolve => {
    wx.sendSms({
      phoneNumber: phones.join(';'),
      content,
      success: () => resolve(true),
      fail: err => {
        if (cancelled(err)) return resolve(false);
        wx.showModal({
          title: '发送给 ' + names.join('、'),
          content,
          confirmText: '发送',
          success: r => resolve(!!r.confirm),
          fail: () => resolve(false)
        });
      }
    });
  });
}

// 定位：resolve(true) 拿到了系统定位；宿主不支持时 resolve(false)，页面按默认上车点处理
function locate() {
  return new Promise(resolve => {
    wx.getLocation({ type: 'gcj02', success: () => resolve(true), fail: () => resolve(false) });
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

// 从通讯录选紧急联系人 / 乘车人：resolve({ name, phone }) / resolve(null)（取消或当前环境不支持，页面里手动填）
function contact() {
  return new Promise(resolve => {
    wx.chooseContact({
      success: r => resolve({ name: r.displayName || '', phone: String(r.phoneNumber || '').replace(/[^\d]/g, '').slice(-11) }),
      fail: err => {
        if (!cancelled(err)) wx.showToast({ title: '当前环境不支持读取通讯录，请手动填写', icon: 'none' });
        resolve(null);
      }
    });
  });
}

function copy(text) {
  wx.setClipboardData({ data: text });
}

module.exports = { pay, call, sms, locate, invoiceTitle, contact, copy };
