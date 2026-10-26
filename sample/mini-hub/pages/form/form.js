// 带参数跳转：选目标（或自填 AppID）、填 path 与 extraData（JSON），校验后调 wx.navigateToMiniProgram
const jump = require('../../utils/jump.js');
const APPID_RE = /^wx[0-9a-f]{16}$/;
const ENVS = ['release', 'trial', 'develop'];
const PRESETS = [
  { label: '云听音乐 · 打开《长河落日》原声', target: 1, path: 'pages/index/index?playlistId=101', extra: '{\n "movieTitle": "长河落日",\n "playlistId": 101\n}' },
  { label: '光影放映厅 · 影片详情', target: 0, path: 'pages/detail/detail?id=2', extra: '{\n "campaign": "hub-demo"\n}' },
  { label: '拾光图库 · 作品详情', target: 2, path: 'pages/detail/detail?id=15', extra: '{}' }
];

Page({
  data: {
    targets: jump.FEATURED.map((a) => a.name).concat(['自定义 AppID']),
    ti: 1, appId: jump.FEATURED[1].appId, custom: false,
    path: PRESETS[0].path, extra: PRESETS[0].extra, extraErr: '', appIdErr: '',
    envs: ENVS, ei: 0, presets: PRESETS, logs: []
  },
  onShow() { this.setData({ logs: jump.logs() }); },
  onTarget(e) {
    const ti = Number(e.detail.value);
    const custom = ti >= jump.FEATURED.length;
    const t = jump.FEATURED[ti];
    this.setData({ ti, custom, appId: custom ? '' : t.appId, path: custom ? '' : t.path, appIdErr: '' });
  },
  onAppId(e) {
    const appId = e.detail.value.trim().toLowerCase();
    this.setData({ appId, appIdErr: appId && !APPID_RE.test(appId) ? 'AppID 是 wx 加 16 位小写十六进制' : '' });
  },
  onPath(e) { this.setData({ path: e.detail.value.trim() }); },
  // 边输边校验 JSON：等点了跳转才报错，用户得回头找是哪个逗号
  onExtra(e) {
    const extra = e.detail.value;
    this.setData({ extra, extraErr: this.checkJson(extra) });
  },
  checkJson(text) {
    if (!text.trim()) return '';
    try {
      const v = JSON.parse(text);
      return v && typeof v === 'object' && !Array.isArray(v) ? '' : 'extraData 要是一个 JSON 对象 {…}';
    } catch (err) {
      return 'JSON 格式不对：' + err.message;
    }
  },
  onEnv(e) { this.setData({ ei: Number(e.detail.value) }); },
  onPreset(e) {
    const p = PRESETS[e.currentTarget.dataset.index];
    const t = jump.FEATURED[p.target];
    this.setData({ ti: p.target, custom: false, appId: t.appId, path: p.path, extra: p.extra, extraErr: '', appIdErr: '' });
  },
  onFormat() {
    if (this.data.extraErr || !this.data.extra.trim()) return;
    this.setData({ extra: JSON.stringify(JSON.parse(this.data.extra), null, 1) });
  },
  onGo() {
    const { appId, path, extra, ti, custom, ei } = this.data;
    if (!APPID_RE.test(appId)) {
      this.setData({ appIdErr: 'AppID 是 wx 加 16 位小写十六进制' });
      wx.showToast({ title: '请先填对 AppID', icon: 'none' });
      return;
    }
    const err = this.checkJson(extra);
    if (err) {
      this.setData({ extraErr: err });
      wx.showToast({ title: 'extraData 不是合法 JSON', icon: 'none' });
      return;
    }
    const target = { appId, path, name: custom ? appId : jump.FEATURED[ti].name, envVersion: ENVS[ei] };
    jump.open(target, extra.trim() ? JSON.parse(extra) : {}, (logs) => this.setData({ logs })).catch(() => {});
  },
  onClearLogs() {
    jump.clearLogs();
    this.setData({ logs: [] });
  }
});
