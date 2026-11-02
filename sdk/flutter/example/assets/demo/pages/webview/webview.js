// web-view 入口：选一个网页打开；网页那页（web.js）收到的 bindmessage / bindload / binderror 放在 globalData 里带回来。
// ?auto=1：自动打开本地 H5 的自测模式（网页加载完自己 postMessage 两次、getEnv、navigateBack），
// 回到这一页时打一行 `NC-CHECK webview …`（tools/android-native-components.sh / ios 同名脚本据此判通过）。
const LINKS = [
  { name: '本地 H5：JSSDK 演示', url: '/h5/bridge.html', icon: 'H5', color: '#07c160' },
  { name: '微信开放文档：web-view', url: 'https://developers.weixin.qq.com/miniprogram/dev/component/web-view.html', icon: '文', color: '#576b95' },
  { name: '打不开的地址（看 binderror）', url: 'https://nonexistent.invalid/', icon: '错', color: '#e64340' }
];

Page({
  data: { links: LINKS, messages: '还没有收到消息', loaded: '-', error: '-' },
  onLoad(q) {
    this.auto = q && q.auto === '1';
    this.chain = q && q.chain === '1';
    if (this.auto) {
      this.autoStarted = Date.now();
      setTimeout(() => wx.navigateTo({ url: '/pages/webview/web?src=' + encodeURIComponent('/h5/bridge.html?auto=1') }), 600);
    }
  },
  onShow() {
    const w = getApp().globalData.web || {};
    this.setData({
      messages: w.messages ? JSON.stringify(w.messages) : '还没有收到消息',
      loaded: w.loaded || '-',
      error: w.error || '-'
    });
    if (this.auto && w.messages) {
      const m = w.messages;
      const env = m.some((x) => x && x.env && x.env.miniprogram === true);
      const ok = m.length >= 2 && env && !!w.loaded;
      console.log('NC-CHECK webview ' + JSON.stringify({ ok: ok, count: m.length, env: env, loaded: !!w.loaded, title: w.title || '', ms: Date.now() - this.autoStarted }));
      this.auto = false;
      if (this.chain) setTimeout(() => wx.redirectTo({ url: '/pages/live/live?auto=1&chain=1' }), 300);
    }
  },
  open(e) {
    getApp().globalData.web = {};
    wx.navigateTo({ url: '/pages/webview/web?src=' + encodeURIComponent(e.currentTarget.dataset.url) });
  }
});
