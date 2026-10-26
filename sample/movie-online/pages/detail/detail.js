// 影片详情：海报头图、评分星级、简介展开收起、演职员横滑、剧照预览、热门短评、想看、听原声（跳云听音乐）
const { request } = require('../../utils/api.js');
const app = getApp();

Page({
  data: { status: 'loading', m: null, wished: false, expanded: false, people: [], starWidth: 0, scoreText: '' },
  onLoad(q) {
    this.id = Number(q.id) || 1;
    this.load();
  },
  load() {
    this.setData({ status: 'loading' });
    request('/api/demo/movies/' + this.id, null, { silent: true }).then((m) => {
      // 导演排在演员前面，横滑列表一起画
      const people = [{ id: 'd', name: m.director.name, role: '导演', avatar: m.director.avatar }].concat(m.cast);
      const comments = m.hotComments.map((c) => Object.assign({}, c, { starWidth: c.star * 20 }));
      this.setData({
        status: 'ok', m, people, comments,
        wished: app.isWished(m.id),
        scoreText: m.score ? m.score.toFixed(1) : '',
        starWidth: m.score * 10,
        headBg: 'linear-gradient(180deg, ' + m.colors[0] + ', ' + m.colors[1] + ')'
      });
      wx.setNavigationBarTitle({ title: m.title });
      wx.setNavigationBarColor({ frontColor: '#ffffff', backgroundColor: m.colors[0] });
    }).catch((err) => {
      this.setData({ status: 'error', errMsg: err.statusCode === 404 ? '这部影片不存在' : '加载失败，请检查网络' });
    });
  },
  onRetry() { this.load(); },
  onToggleSummary() { this.setData({ expanded: !this.data.expanded }); },
  onWish() {
    const on = app.toggleWish(this.data.m);
    this.setData({ wished: on });
    wx.showToast({ title: on ? '已加入想看，上映提醒你' : '已取消想看', icon: 'none' });
  },
  // 剧照：urls 给全部剧照，预览层里能左右翻
  onStill(e) {
    const stills = this.data.m.stills;
    wx.previewImage({ urls: stills, current: stills[e.currentTarget.dataset.index] });
  },
  onPoster() {
    wx.previewImage({ urls: [this.data.m.poster].concat(this.data.m.stills), current: this.data.m.poster });
  },
  onComments() {
    wx.navigateTo({ url: '/pages/comments/comments?id=' + this.id + '&title=' + encodeURIComponent(this.data.m.title) });
  },
  onWrite() {
    wx.showToast({ title: '演示版暂不支持发表短评', icon: 'none' });
  },
  // 听原声：打开云听音乐的首页，query 带歌单 id（对方首页读到就直接打开歌单），extraData 带片名
  onSoundtrack() {
    const m = this.data.m;
    wx.navigateToMiniProgram({
      appId: app.globalData.musicAppId,
      path: 'pages/index/index?playlistId=' + m.soundtrackPlaylistId,
      extraData: { from: 'movie-online', fromName: '光影放映厅', movieId: m.id, movieTitle: m.title, playlistId: m.soundtrackPlaylistId },
      success() { console.log('[movie-online] 已跳到云听音乐'); },
      fail(err) {
        wx.showModal({
          title: '没能打开云听音乐',
          content: (err && err.errMsg) || '宿主不支持小程序跳转',
          showCancel: false
        });
      }
    });
  }
});
