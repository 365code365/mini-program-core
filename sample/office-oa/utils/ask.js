// 要一段文字：优先 wx.showModal({ editable: true, placeholderText })（微信 2.17.1+）。
// 本引擎桌面端的 showModal 没有输入框、回调里没有 content：记下来，之后直接用页内的 input-dialog 组件。
// 页面要在 wxml 放 <input-dialog show="{{dlg.show}}" title="{{dlg.title}}" placeholder="{{dlg.placeholder}}" bindconfirm="onDlgOk" bindcancel="onDlgCancel"/>，
// 并把 onDlgOk / onDlgCancel 转给 ask.ok / ask.cancel。
const K = 'oa_modal_no_editable';

function viaDialog(page, o) {
  return new Promise(resolve => {
    page.__askResolve = resolve;
    page.setData({ dlg: { show: true, title: o.title, placeholder: o.placeholder || '', required: !!o.required } });
  });
}

function text(page, o) {
  if (wx.getStorageSync(K)) return viaDialog(page, o);
  return new Promise(resolve => {
    wx.showModal({
      title: o.title, editable: true, placeholderText: o.placeholder || '', content: '',
      confirmText: o.confirmText || '确定',
      success: res => {
        if (!res.confirm) return resolve(null);
        if (typeof res.content !== 'string') {
          wx.setStorageSync(K, 1);
          return viaDialog(page, o).then(resolve);
        }
        if (o.required && !res.content.trim()) {
          wx.showToast({ title: '请填写后再提交', icon: 'none' });
          return resolve(null);
        }
        resolve(res.content.trim());
      },
      fail: () => viaDialog(page, o).then(resolve)
    });
  });
}

function ok(page, e) {
  page.setData({ 'dlg.show': false });
  const r = page.__askResolve; page.__askResolve = null;
  r && r(e.detail.value);
}
function cancel(page) {
  page.setData({ 'dlg.show': false });
  const r = page.__askResolve; page.__askResolve = null;
  r && r(null);
}

module.exports = { text, ok, cancel };
