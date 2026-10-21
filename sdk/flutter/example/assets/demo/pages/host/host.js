// 宿主 tab：凡是要 Flutter 出力的都在这里 —— hostApis、原生组件、剪贴板 / 振动事件、
// tabBar（引擎画的原生 tabBar）、前后台切换带来的生命周期。
const HOST_APIS = ['chooseImage', 'login', 'getUserProfile', 'getLocation', 'scanCode', 'requestPayment', 'makePhoneCall'];

// 每个 API 的调用参数（宿主那边能在 args 里看到）
const ARGS = {
  chooseImage: { count: 2, sizeType: ['compressed'], sourceType: ['album', 'camera'] },
  getUserProfile: { desc: '用于演示' },
  getLocation: { type: 'gcj02' },
  scanCode: { onlyFromCamera: false },
  requestPayment: { timeStamp: '1700000000', nonceStr: 'demo', package: 'prepay_id=demo', signType: 'RSA', paySign: 'x', total: 1990 },
  makePhoneCall: { phoneNumber: '10086' },
  chooseMedia: { count: 1, mediaType: ['image'] },
  chooseLocation: {}
};

function brief(obj) {
  const copy = {};
  Object.keys(obj).forEach((k) => { if (k !== 'errMsg') copy[k] = obj[k]; });
  const s = JSON.stringify(copy);
  return s.length > 160 ? s.slice(0, 160) + '…' : s;
}

Page({
  data: {
    hostApis: HOST_APIS,
    hostResult: '点上面的按钮',
    lat: 39.908823,
    lng: 116.39747,
    mapResult: '点地图上的标记，宿主回传 markertap',
    clipResult: '',
    tabResult: '',
    barHidden: false,
    darkBar: false,
    renamed: false,
    appShow: 0,
    appHide: 0,
    pageShow: 0,
    lifecycle: [],
    sys: []
  },
  onLoad() {
    this.loadSystemInfo();
  },
  onShow() {
    const g = getApp().globalData;
    this.setData({
      pageShow: this.data.pageShow + 1,
      appShow: g.showCount,
      appHide: g.hideCount,
      lifecycle: g.lifecycle.slice().reverse()
    });
  },

  // ── hostApis ──
  callHost(e) {
    const api = e.currentTarget.dataset.api;
    const started = Date.now();
    this.setData({ hostResult: 'wx.' + api + ' 调用中…' });
    wx[api](Object.assign({}, ARGS[api] || {}, {
      success: (r) => this.setData({ hostResult: 'wx.' + api + ' ✓ ' + r.errMsg + '（' + (Date.now() - started) + 'ms）\n' + brief(r) }),
      fail: (r) => this.setData({ hostResult: 'wx.' + api + ' ✗ ' + r.errMsg })
    }));
  },

  // ── 原生 map ──
  onMarker(e) {
    this.setData({ mapResult: '宿主回传 markertap：' + JSON.stringify(e.detail) });
  },

  // ── 剪贴板 / 振动 ──
  copy() {
    const d = new Date();
    wx.setClipboardData({ data: 'mini_render @' + d.getHours() + ':' + d.getMinutes() + ':' + d.getSeconds() });
  },
  paste() {
    wx.getClipboardData({ success: (r) => this.setData({ clipResult: '读回：' + r.data }) });
  },
  vibShort() {
    wx.vibrateShort({ type: 'heavy', success: () => this.setData({ clipResult: 'vibrateShort ok（宿主收到 vibrate 事件）' }) });
  },
  vibLong() {
    wx.vibrateLong({ success: () => this.setData({ clipResult: 'vibrateLong ok' }) });
  },

  // ── tabBar ──（结果在下一帧回来）
  tabCall(name, opt) {
    wx[name](Object.assign({}, opt, {
      success: () => this.setData({ tabResult: name + ' ok' }),
      fail: (e) => this.setData({ tabResult: name + ' 失败：' + e.errMsg })
    }));
  },
  badge() { this.tabCall('setTabBarBadge', { index: 1, text: String(Math.floor(Math.random() * 120)) }); },
  unbadge() { this.tabCall('removeTabBarBadge', { index: 1 }); },
  dot() { this.tabCall('showTabBarRedDot', { index: 2 }); },
  undot() { this.tabCall('hideTabBarRedDot', { index: 2 }); },
  toggleStyle() {
    const dark = !this.data.darkBar;
    this.setData({ darkBar: dark });
    this.tabCall('setTabBarStyle', dark
      ? { backgroundColor: '#1f1f1f', color: '#8a8a8a', selectedColor: '#ffc300', borderStyle: 'white' }
      : { backgroundColor: '#ffffff', color: '#7a7e83', selectedColor: '#07c160', borderStyle: 'black' });
  },
  toggleItem() {
    const renamed = !this.data.renamed;
    this.setData({ renamed });
    this.tabCall('setTabBarItem', { index: 3, text: renamed ? '宿主 ✓' : '宿主' });
  },
  toggleBar() {
    const hidden = !this.data.barHidden;
    this.setData({ barHidden: hidden });
    this.tabCall(hidden ? 'hideTabBar' : 'showTabBar', {});
  },

  // ── 系统信息 ──
  loadSystemInfo() {
    const w = wx.getWindowInfo();
    const d = wx.getDeviceInfo();
    const a = wx.getAppBaseInfo();
    const s = wx.getSystemSetting();
    const auth = wx.getAppAuthorizeSetting();
    this.setData({
      sys: [
        { k: '窗口', v: w.windowWidth + '×' + w.windowHeight + ' @' + w.pixelRatio + 'x' },
        { k: '屏幕', v: w.screenWidth + '×' + w.screenHeight },
        { k: '状态栏高度', v: String(w.statusBarHeight) },
        { k: '安全区', v: [w.safeArea.top, w.safeArea.bottom, w.safeArea.left, w.safeArea.right].join(' / ') },
        { k: '设备', v: [d.brand, d.model, d.system].filter(Boolean).join(' ') },
        { k: '平台', v: d.platform },
        { k: '语言 / 主题', v: a.language + ' / ' + (a.theme || 'light') },
        { k: '基础库', v: String(a.SDKVersion) },
        { k: '定位 / WiFi 开关', v: s.locationEnabled + ' / ' + s.wifiEnabled },
        { k: '相机授权', v: String(auth.cameraAuthorized) },
        { k: '启动场景', v: String(wx.getLaunchOptionsSync().scene) }
      ]
    });
  }
});
