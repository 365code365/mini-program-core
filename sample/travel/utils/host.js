// 要宿主才能做的事（剪贴板）。微信、接了 hostApis 的宿主 App 里走系统剪贴板；
// 读不到（宿主没实现 / 用户拒绝）时 resolve('')，调用方退回手动粘贴。写法照 sample/coffee/utils/host.js。
function cancelled(err) {
  return /cancel/.test((err && err.errMsg) || '');
}
// 写剪贴板：成功后微信自己会弹「内容已复制」
function copy(text) {
  return new Promise(resolve => {
    wx.setClipboardData({
      data: text,
      success: () => resolve(true),
      fail: err => {
        if (!cancelled(err)) wx.showModal({ title: '复制失败', content: '当前环境不能写剪贴板，请长按文字手动复制', showCancel: false, confirmText: '知道了' });
        resolve(false);
      }
    });
  });
}
// 读剪贴板：resolve(文字)，读不到 resolve('')
function paste() {
  return new Promise(resolve => {
    if (typeof wx.getClipboardData !== 'function') { resolve(''); return; }
    wx.getClipboardData({ success: r => resolve(String(r.data || '')), fail: () => resolve('') });
  });
}
module.exports = { copy, paste };
