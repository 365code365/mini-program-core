// 公共回显服务：GET / POST 原样回传请求内容，/status/404 回 404，/image/png 回一张图。
// 换成自己的服务只改这一行。
const BASE = 'https://httpbin.org';

function cut(s, n) {
  s = typeof s === 'string' ? s : JSON.stringify(s);
  return s.length > n ? s.slice(0, n) + '…' : s;
}

Page({
  data: {
    base: BASE,
    reqResult: '点上面的按钮发请求',
    tempFile: '',
    dlResult: '',
    remoteImg: BASE + '/image/jpeg',
    remoteInfo: '加载中…'
  },
  started: 0,
  begin(label) {
    this.started = Date.now();
    this.setData({ reqResult: label + ' 请求中…' });
  },
  ms() {
    return (Date.now() - this.started) + 'ms';
  },
  get() {
    this.begin('GET');
    wx.request({
      url: BASE + '/get',
      data: { from: 'mini_render', 中文: '参数' },
      header: { 'X-Demo': '1' },
      success: (r) => this.setData({ reqResult: 'GET ' + r.statusCode + '（' + this.ms() + '）\nargs = ' + cut(r.data.args, 120) }),
      fail: (e) => this.setData({ reqResult: 'GET fail：' + e.errMsg })
    });
  },
  post() {
    this.begin('POST');
    wx.request({
      url: BASE + '/post',
      method: 'POST',
      header: { 'content-type': 'application/json' },
      data: { cart: [{ sku: 'A1', n: 2 }], total: 5600 },
      success: (r) => this.setData({ reqResult: 'POST ' + r.statusCode + '（' + this.ms() + '）\n服务端收到 json = ' + cut(r.data.json, 140) }),
      fail: (e) => this.setData({ reqResult: 'POST fail：' + e.errMsg })
    });
  },
  status404() {
    this.begin('404');
    wx.request({
      url: BASE + '/status/404',
      success: (r) => this.setData({ reqResult: '走的是 success，statusCode = ' + r.statusCode + '（' + this.ms() + '）—— HTTP 错误码不算失败，与微信一致' }),
      fail: (e) => this.setData({ reqResult: '404 fail：' + e.errMsg })
    });
  },
  abort() {
    this.begin('abort');
    const task = wx.request({
      url: BASE + '/delay/2',
      success: () => this.setData({ reqResult: '不该走到 success' }),
      fail: (e) => this.setData({ reqResult: 'abort 后走 fail：' + e.errMsg }),
      // 被 abort 的请求不回 success，complete 照样调
      complete: (r) => this.setData({ reqResult: 'abort：没有 success，complete 收到 ' + (r && r.errMsg) + '（' + this.ms() + '）' })
    });
    task.abort();
  },
  download() {
    this.setData({ dlResult: '下载中…' });
    wx.downloadFile({
      url: BASE + '/image/png',
      success: (r) => this.setData({ tempFile: r.tempFilePath, dlResult: 'statusCode ' + r.statusCode + '\n' + r.tempFilePath }),
      fail: (e) => this.setData({ dlResult: 'fail：' + e.errMsg })
    });
  },
  upload() {
    if (!this.data.tempFile) {
      wx.showToast({ title: '先下载一张', icon: 'none' });
      return;
    }
    this.setData({ dlResult: '上传中…' });
    wx.uploadFile({
      url: BASE + '/post',
      filePath: this.data.tempFile,
      name: 'file',
      formData: { note: 'from mini_render' },
      success: (r) => {
        let size = '';
        try { size = '，服务端收到 form=' + cut(JSON.parse(r.data).form, 60); } catch (err) { size = ''; }
        this.setData({ dlResult: 'uploadFile ' + r.statusCode + '（data 是字符串，长度 ' + r.data.length + '）' + size });
      },
      fail: (e) => this.setData({ dlResult: 'upload fail：' + e.errMsg })
    });
  },
  onRemoteLoad(e) {
    this.setData({ remoteInfo: '远程图加载完成 ' + e.detail.width + '×' + e.detail.height });
  },
  onRemoteErr(e) {
    this.setData({ remoteInfo: '远程图加载失败：' + e.detail.errMsg });
  }
});
