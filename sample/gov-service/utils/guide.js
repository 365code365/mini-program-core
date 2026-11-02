// 办事指南 PDF：wx.downloadFile 下到临时文件，再 wx.openDocument 预览。
// 网络不通 / 服务端不是 200 / 宿主不支持预览时都降级成「指南要点」弹窗，并告诉用户去哪里拿纸质版。
const { getItem } = require('./items');

const BASE = 'https://zwfw.yunchuan-demo.cn/guide/';

function fallback(item, why) {
  const text = item
    ? `${item.name}\n承诺时限：${item.promise ? item.promise + ' 个工作日' : '即时办结'}\n收费：${item.fee}\n办理地点：${item.window}`
    : '可在云川市政务服务中心一楼咨询台领取纸质版《办事指南汇编》。';
  wx.showModal({ title: `指南下载失败（${why}）`, content: text, showCancel: false, confirmText: '知道了' });
}

function open(itemId) {
  const item = itemId ? getItem(itemId) : null;
  const url = BASE + (item ? item.id : 'all') + '.pdf';
  wx.showLoading({ title: '正在下载指南' });
  wx.downloadFile({
    url, timeout: 8000,
    success: (r) => {
      wx.hideLoading();
      if (r.statusCode !== 200) { fallback(item, '服务器返回 ' + r.statusCode); return; }
      wx.openDocument({
        filePath: r.tempFilePath, fileType: 'pdf', showMenu: true,
        fail: () => fallback(item, '当前设备无法预览 PDF')
      });
    },
    fail: () => { wx.hideLoading(); fallback(item, '网络不可用'); }
  });
}

module.exports = { open };
