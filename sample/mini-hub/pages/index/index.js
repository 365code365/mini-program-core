// 跳转中心首页：启动参数（被谁打开）、精选小程序、<navigator target="miniProgram"> 写法、商店全部小程序（分页）
const api = require('../../utils/api.js');
const jump = require('../../utils/jump.js');
const app = getApp();
const PAGE_SIZE = 20;

function pretty(obj) {
  try { return JSON.stringify(obj, null, 1); } catch (e) { return String(obj); }
}

Page({
  data: {
    launch: null, enter: null, featured: [], apps: [], total: 0, status: 'loading', more: 'idle', logs: [],
    // <navigator extra-data> 用 JSON 字符串：本引擎把属性当字符串传，接收方（云听音乐）两种都认
    navExtra: '{"from":"mini-hub","via":"navigator","playlistId":105}',
    navExtra2: '{"from":"mini-hub","via":"navigator"}'
  },
  onLoad() {
    this.page = 0;
    this.setData({ featured: jump.FEATURED.map((a) => Object.assign({}, a, { icon: api.BASE + '/api/apps/' + a.appId + '/icon', iconFail: false, initial: a.name[0] })) });
    this.fetch(1);
  },
  onShow() {
    const l = wx.getLaunchOptionsSync();
    const e = app.globalData.enterOptions || l;
    const ref = l.referrerInfo || {};
    this.setData({
      launch: {
        scene: l.scene, path: l.path || '(默认首页)', query: pretty(l.query || {}),
        hasRef: !!ref.appId, refAppId: ref.appId || '', refExtra: pretty(ref.extraData || {})
      },
      enter: { scene: e.scene, refAppId: (e.referrerInfo || {}).appId || '' },
      logs: jump.logs().slice(0, 5)
    });
  },
  onPullDownRefresh() { this.fetch(1).then(() => wx.stopPullDownRefresh()); },
  onReachBottom() { if (this.data.more === 'idle') this.fetch(this.page + 1); },
  // 商店列表走 /api/apps 的分页参数（不带参数是全量，给 App 用）
  fetch(page) {
    if (page > 1) this.setData({ more: 'loading' });
    return api.request('/api/apps', { page, pageSize: PAGE_SIZE }, { silent: true }).then((r) => {
      this.page = r.page;
      const items = (r.apps || []).map((a) => ({
        appId: a.appId, name: a.name, description: a.description || '', icon: a.iconUrl ? api.abs(a.iconUrl) : '',
        initial: (a.name || '?')[0], category: a.category, iconFail: !a.iconUrl
      }));
      const apps = page === 1 ? items : this.data.apps.concat(items);
      const hasMore = r.page * r.pageSize < r.total;
      this.setData({ apps, total: r.total, status: apps.length ? 'ok' : 'empty', more: hasMore ? 'idle' : 'end' });
    }).catch(() => {
      this.setData(page === 1 ? { status: 'error' } : { more: 'error' });
    });
  },
  onRetry() { this.setData({ status: 'loading' }); this.fetch(1); },
  onRetryMore() { this.fetch(this.page + 1); },
  // 图标加载失败（服务器上没这个小程序 / 没传图标）：换成首字色块
  onIconErr(e) {
    const { list, index } = e.currentTarget.dataset;
    this.setData({ [list + '[' + index + '].iconFail']: true });
  },
  onOpenFeatured(e) {
    const a = this.data.featured[e.currentTarget.dataset.index];
    jump.open(a, { scene: 'featured' }, (logs) => this.setData({ logs: logs.slice(0, 5) })).catch(() => {});
  },
  onOpenStore(e) {
    const a = this.data.apps[e.currentTarget.dataset.index];
    if (!a.appId) { wx.showToast({ title: '这个小程序还没有 AppID', icon: 'none' }); return; }
    jump.open({ appId: a.appId, name: a.name, path: '' }, { scene: 'store-list' }, (logs) => this.setData({ logs: logs.slice(0, 5) })).catch(() => {});
  },
  onCopy(e) {
    wx.setClipboardData({ data: e.currentTarget.dataset.text });
  },
  onClearLogs() {
    jump.clearLogs();
    this.setData({ logs: [] });
  },
  onForm() { wx.switchTab({ url: '/pages/form/form' }); }
});
