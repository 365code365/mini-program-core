// 云听音乐：歌单 / 榜单 / 歌词都从演示接口拉；播放器状态在 globalData.player（utils/player.js）。
// 被别的小程序打开时（光影放映厅「听原声」、跳转中心），onLaunch / onShow 的 options.referrerInfo
// 带对方的 appId 与 extraData：记下来给首页画「来自 xxx」提示条，query / extraData 里有歌单就直接打开。
const KNOWN_APPS = {
  wxaaf1983a0caced0c: '光影放映厅',
  wxd59107b7765c4030: '小程序跳转中心',
  wx6158cbd043d9faf1: '拾光图库'
};

App({
  globalData: { player: null, referrer: null, pendingPlaylistId: 0, chartId: 1 },
  onLaunch(options) { this.handleEnter(options); },
  // 热启动（已经在后台，又被别的小程序拉起）只走 onShow，所以两处都要处理
  onShow(options) { this.handleEnter(options); },
  handleEnter(options) {
    options = options || {};
    const ref = options.referrerInfo || {};
    let extra = ref.extraData;
    // <navigator target="miniProgram" extra-data> 传过来的可能是 JSON 字符串
    if (typeof extra === 'string') { try { extra = JSON.parse(extra); } catch (e) { extra = { raw: extra }; } }
    extra = extra || {};
    if (ref.appId) {
      this.globalData.referrer = {
        appId: ref.appId,
        name: extra.fromName || KNOWN_APPS[ref.appId] || ref.appId,
        movieTitle: extra.movieTitle || '',
        extraData: extra,
        at: Date.now()
      };
    }
    const q = options.query || {};
    const pid = Number(q.playlistId || extra.playlistId || 0);
    if (pid) this.globalData.pendingPlaylistId = pid;
  },
  // 返回上一个小程序：宿主不支持或不是被小程序打开的，会走 fail
  backToReferrer() {
    const ref = this.globalData.referrer;
    wx.navigateBackMiniProgram({
      extraData: { from: 'music-online', listened: true },
      success: () => { this.globalData.referrer = null; },
      fail: (err) => {
        wx.showToast({ title: '没能返回' + (ref ? '「' + ref.name + '」' : '') + '：' + ((err && err.errMsg) || '宿主不支持'), icon: 'none' });
      }
    });
  }
});
