// 承载网页的那一页：只有一个 <web-view>（官方：每页只能有一个，铺满整页）。收到的事件放进 globalData，回到入口页显示
Page({
  data: { src: '' },
  onLoad(q) {
    this.setData({ src: decodeURIComponent((q && q.src) || '/h5/bridge.html') });
    getApp().globalData.web = getApp().globalData.web || {};
  },
  onMessage(e) {
    // e.detail.data 是数组：这段时间里网页每次 postMessage 的 data
    const w = getApp().globalData.web;
    w.messages = (w.messages || []).concat(e.detail.data || []);
    console.log('[web-view] bindmessage ' + JSON.stringify(e.detail.data));
  },
  onWebLoad(e) {
    getApp().globalData.web.loaded = e.detail.src;
    console.log('[web-view] bindload ' + e.detail.src);
  },
  onWebError(e) {
    getApp().globalData.web.error = (e.detail.errMsg || 'error') + ' ' + (e.detail.src || '');
    console.log('[web-view] binderror ' + JSON.stringify(e.detail));
  }
});
