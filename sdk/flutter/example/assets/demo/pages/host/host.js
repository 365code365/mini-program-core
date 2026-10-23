// 宿主 tab：凡是要 Flutter 出力的都在这里 —— 媒体与文件（插件自带）、hostApis、原生组件、
// 剪贴板 / 振动事件、tabBar（引擎画的原生 tabBar）、前后台切换带来的生命周期。
//
// hostApis 按微信的分类排；每个都由例子的 Flutter 侧（example/lib/host_apis.dart）给演示实现。
// 桌面窗体只接了媒体与文件那几个，这一组在桌面上都会立刻 fail（errMsg 写明原因）。
const HOST_GROUPS = [
  { name: '账号与授权', apis: ['login', 'checkSession', 'getUserProfile', 'getUserInfo', 'authorize', 'openSetting'] },
  { name: '位置与地址', apis: ['getLocation', 'chooseLocation', 'openLocation', 'chooseAddress'] },
  { name: '支付与消息', apis: ['requestPayment', 'requestSubscribeMessage', 'chooseInvoice', 'openCustomerServiceChat'] },
  { name: '设备', apis: ['scanCode', 'makePhoneCall', 'chooseContact', 'addPhoneContact', 'startSoterAuthentication', 'getWeRunData'] },
  { name: '跳转与分享', apis: ['navigateToMiniProgram', 'navigateBackMiniProgram', 'getShareInfo'] }
];
// 每个 API 的调用参数（宿主那边能在 args 里看到）
const ARGS = {
  login: { timeout: 5000 },
  getUserProfile: { desc: '用于演示' },
  getUserInfo: { withCredentials: false, lang: 'zh_CN' },
  authorize: { scope: 'scope.userLocation' },
  getLocation: { type: 'gcj02' },
  chooseLocation: { latitude: 39.908823, longitude: 116.39747 },
  openLocation: { latitude: 39.908823, longitude: 116.39747, name: '天安门', address: '北京市东城区东长安街', scale: 16 },
  requestPayment: { timeStamp: '1700000000', nonceStr: 'demo', package: 'prepay_id=demo', signType: 'RSA', paySign: 'x', total: 1990 },
  requestSubscribeMessage: { tmplIds: ['demo_tmpl_order', 'demo_tmpl_ship'] },
  openCustomerServiceChat: { extInfo: { url: 'https://work.weixin.qq.com/kfid/demo' }, corpId: 'demo_corp' },
  scanCode: { onlyFromCamera: false },
  makePhoneCall: { phoneNumber: '10086' },
  addPhoneContact: { firstName: '小程序', mobilePhoneNumber: '13800000000', organization: 'mini_render' },
  startSoterAuthentication: { requestAuthModes: ['fingerPrint'], challenge: 'demo', authContent: '验证指纹以继续' },
  navigateToMiniProgram: { appId: 'wx0000000000000000', path: 'pages/index/index', extraData: { from: 'host-demo' } },
  navigateBackMiniProgram: { extraData: { ok: true } },
  getShareInfo: { shareTicket: 'demo_ticket' }
};
function brief(obj) {
  const copy = {};
  Object.keys(obj).forEach((k) => { if (k !== 'errMsg') copy[k] = obj[k]; });
  const s = JSON.stringify(copy);
  return s.length > 200 ? s.slice(0, 200) + '…' : s;
}
function kb(n) {
  if (n < 1024) return n + 'B';
  return n >= 1048576 ? (n / 1048576).toFixed(1) + 'MB' : (n / 1024).toFixed(1) + 'KB';
}
Page({
  data: {
    hostGroups: HOST_GROUPS,
    hostResult: '点上面的按钮',
    media: [],
    hasVideo: false,
    files: [],
    mediaResult: '选到的图片、视频封面显示在上面（点一下 previewImage）',
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
  // ── 媒体与文件（插件自带）──
  // 每个回包都是 wxfile://tmp/… 虚拟路径：直接给 <image src>，也能 getImageInfo / saveFile / uploadFile
  pick(api, opts, toItems) {
    const started = Date.now();
    this.setData({ mediaResult: 'wx.' + api + ' 调用中…' });
    wx[api](Object.assign({}, opts, {
      success: (r) => {
        const items = toItems(r);
        const media = items.concat(this.data.media).slice(0, 9);
        this.setData({
          media,
          hasVideo: media.some((m) => m.kind === 'video'),
          mediaResult: 'wx.' + api + ' ✓（' + (Date.now() - started) + 'ms）\n' + brief(r)
        });
      },
      fail: (r) => this.setData({ mediaResult: 'wx.' + api + ' ✗ ' + r.errMsg })
    }));
  },
  chooseImage() {
    this.pick('chooseImage', { count: 9, sizeType: ['original', 'compressed'], sourceType: ['album', 'camera'] },
      (r) => r.tempFiles.map((f) => ({ src: f.path, path: f.path, kind: 'image', label: kb(f.size) })));
  },
  chooseMedia() {
    this.pick('chooseMedia', { count: 9, mediaType: ['image', 'video'], sourceType: ['album', 'camera'], maxDuration: 30 },
      (r) => r.tempFiles.map((f) => f.fileType === 'video'
        ? { src: f.thumbTempFilePath, path: f.tempFilePath, kind: 'video', label: f.duration.toFixed(1) + 's' }
        : { src: f.tempFilePath, path: f.tempFilePath, kind: 'image', label: kb(f.size) }));
  },
  chooseVideo() {
    this.pick('chooseVideo', { sourceType: ['album', 'camera'], maxDuration: 30, compressed: true },
      (r) => [{ src: r.thumbTempFilePath || '', path: r.tempFilePath, kind: 'video', label: r.duration.toFixed(1) + 's' }]);
  },
  chooseFile() {
    this.setData({ mediaResult: 'wx.chooseMessageFile 调用中…' });
    wx.chooseMessageFile({
      count: 5,
      type: 'all',
      success: (r) => this.setData({
        files: r.tempFiles.map((f) => ({ name: f.name, path: f.path, label: f.type + ' · ' + kb(f.size) })),
        mediaResult: 'wx.chooseMessageFile ✓ ' + r.tempFiles.length + ' 个文件（点一下 openDocument）\n' + brief(r)
      }),
      fail: (r) => this.setData({ mediaResult: 'wx.chooseMessageFile ✗ ' + r.errMsg })
    });
  },
  preview(e) {
    const images = this.data.media.filter((m) => m.kind === 'image').map((m) => m.path);
    const item = this.data.media[e.currentTarget.dataset.index];
    if (item.kind !== 'image') {
      this.setData({ mediaResult: '视频：' + item.path });
      return;
    }
    wx.previewImage({ urls: images, current: item.path });
  },
  firstOf(kind) {
    const m = this.data.media.find((x) => x.kind === kind);
    if (!m) this.setData({ mediaResult: '先选一个' + (kind === 'video' ? '视频' : '图片') });
    return m;
  },
  imageInfo() {
    const m = this.firstOf('image');
    if (!m) return;
    wx.getImageInfo({
      src: m.path,
      success: (r) => this.setData({ mediaResult: 'getImageInfo ✓ ' + r.width + '×' + r.height + ' ' + r.type }),
      fail: (r) => this.setData({ mediaResult: 'getImageInfo ✗ ' + r.errMsg })
    });
  },
  saveAlbum() {
    const m = this.firstOf('image');
    if (!m) return;
    wx.saveImageToPhotosAlbum({
      filePath: m.path,
      success: () => this.setData({ mediaResult: 'saveImageToPhotosAlbum ✓（去系统相册看看）' }),
      fail: (r) => this.setData({ mediaResult: 'saveImageToPhotosAlbum ✗ ' + r.errMsg })
    });
  },
  saveVideo() {
    const m = this.firstOf('video');
    if (!m) return;
    wx.saveVideoToPhotosAlbum({
      filePath: m.path,
      success: () => this.setData({ mediaResult: 'saveVideoToPhotosAlbum ✓' }),
      fail: (r) => this.setData({ mediaResult: 'saveVideoToPhotosAlbum ✗ ' + r.errMsg })
    });
  },
  // 临时文件存成本地文件（wxfile://store/…）：证明选到的文件走得通 FileSystemManager 那一套
  persist() {
    const m = this.data.media[0];
    if (!m) return;
    wx.saveFile({
      tempFilePath: m.path,
      success: (r) => this.setData({ mediaResult: 'saveFile ✓ ' + r.savedFilePath }),
      fail: (r) => this.setData({ mediaResult: 'saveFile ✗ ' + r.errMsg })
    });
  },
  openDoc(e) {
    const f = this.data.files[e.currentTarget.dataset.index];
    wx.openDocument({
      filePath: f.path,
      showMenu: true,
      success: () => this.setData({ mediaResult: 'openDocument ✓ ' + f.name }),
      fail: (r) => this.setData({ mediaResult: 'openDocument ✗ ' + r.errMsg })
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
