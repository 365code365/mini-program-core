Page({
  data: {
    visits: 0,
    key: 'demo:note',
    value: '你好，storage',
    syncResult: '设一个值，返回后再进来、甚至关掉 App 再开，读出来还在',
    asyncResult: '对象、数字、布尔存进去取出来类型不变',
    info: { keys: [], currentSize: 0, limitSize: 0 }
  },
  onLoad() {
    // 读 + 写：缺 key 时 getStorageSync 返回空字符串（与微信一致），Number('') 是 0
    const visits = Number(wx.getStorageSync('demo:visits')) + 1;
    wx.setStorageSync('demo:visits', visits);
    this.setData({ visits });
    this.refreshInfo();
  },
  refreshInfo() {
    const i = wx.getStorageInfoSync();
    this.setData({ info: { keys: i.keys, currentSize: i.currentSize, limitSize: i.limitSize } });
  },
  onKey(e) { this.setData({ key: e.detail.value }); },
  onValue(e) { this.setData({ value: e.detail.value }); },
  set() {
    wx.setStorageSync(this.data.key, this.data.value);
    this.setData({ syncResult: '已写入 ' + this.data.key });
    this.refreshInfo();
  },
  get() {
    const v = wx.getStorageSync(this.data.key);
    this.setData({ syncResult: 'getStorageSync("' + this.data.key + '") → ' + JSON.stringify(v) + '（' + typeof v + '）' });
  },
  remove() {
    wx.removeStorageSync(this.data.key);
    this.setData({ syncResult: '已删除 ' + this.data.key });
    this.refreshInfo();
  },
  clearAll() {
    wx.showModal({
      title: '清空缓存',
      content: '会删掉本小程序的全部 storage（包括打开次数）',
      success: (r) => {
        if (!r.confirm) return;
        wx.clearStorageSync();
        this.setData({ syncResult: '已清空', visits: 0 });
        this.refreshInfo();
      }
    });
  },
  setObj() {
    const obj = { id: 7, name: '购物车', items: [{ sku: 'A1', n: 2 }], paid: false, at: Date.now() };
    wx.setStorage({
      key: 'demo:cart',
      data: obj,
      success: () => {
        this.setData({ asyncResult: 'setStorage 成功：' + JSON.stringify(obj) });
        this.refreshInfo();
      }
    });
  },
  getObj() {
    wx.getStorage({
      key: 'demo:cart',
      success: (r) => this.setData({ asyncResult: 'getStorage：items[0].n 是 ' + typeof r.data.items[0].n + '，paid 是 ' + typeof r.data.paid + '\n' + JSON.stringify(r.data) }),
      fail: (e) => this.setData({ asyncResult: 'getStorage fail：' + e.errMsg + '（先点左边写入）' })
    });
  },
  getMissing() {
    const v = wx.getStorageSync('demo:does-not-exist');
    const head = '同步读缺 key → ' + JSON.stringify(v) + '（空字符串）\n异步读缺 key → ';
    wx.getStorage({
      key: 'demo:does-not-exist',
      success: (r) => this.setData({ asyncResult: head + 'success，data=' + JSON.stringify(r.data) }),
      fail: (e) => this.setData({ asyncResult: head + 'fail：' + e.errMsg })
    });
  },
  removeObj() {
    wx.removeStorage({
      key: 'demo:cart',
      success: () => {
        this.setData({ asyncResult: 'removeStorage demo:cart 成功' });
        this.refreshInfo();
      }
    });
  }
});
