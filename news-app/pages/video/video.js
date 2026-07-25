// 头条新闻 · 视频频道
var ALL_VIDEOS = {
  1: [
    { id: 501, title: '实拍：国产大飞机完成高原试飞全过程', author: '航空观察', src: '/assets/oceans.mp4', cover: '/assets/p_phone.jpg', duration: '03:24', plays: '86.2万', comments: 1263 },
    { id: 502, title: '三分钟看懂新能源车电池技术路线之争', author: '汽车实验室', src: '/assets/oceans.mp4', cover: '/assets/p_watch.jpg', duration: '02:58', plays: '42.7万', comments: 486 },
    { id: 503, title: '老街区改造前后对比，变化令人惊叹', author: '城市影像', src: '/assets/oceans.mp4', cover: '/assets/p_coffee.jpg', duration: '05:12', plays: '31.5万', comments: 274 }
  ],
  2: [
    { id: 511, title: '联赛绝杀时刻多机位回放', author: '体育直播间', src: '/assets/oceans.mp4', cover: '/assets/p_sneakers.jpg', duration: '01:46', plays: '128.4万', comments: 3521 },
    { id: 512, title: '游泳世锦赛夺金瞬间全记录', author: '竞技体育', src: '/assets/oceans.mp4', cover: '/assets/p_thermos.jpg', duration: '04:03', plays: '57.9万', comments: 912 }
  ],
  3: [
    { id: 521, title: '暑期档热门影片幕后花絮合集', author: '娱乐观察', src: '/assets/oceans.mp4', cover: '/assets/p_cake.jpg', duration: '06:35', plays: '73.1万', comments: 1408 },
    { id: 522, title: '综艺现场：嘉宾即兴表演引爆全场', author: '视听前沿', src: '/assets/oceans.mp4', cover: '/assets/p_lipstick.jpg', duration: '02:21', plays: '25.6万', comments: 337 }
  ]
};

Page({
  data: {
    currentFilter: 1,
    filters: [
      { id: 1, name: '推荐' },
      { id: 2, name: '体育' },
      { id: 3, name: '娱乐' }
    ],
    videos: ALL_VIDEOS[1],
    playingId: 0,
    autoPlay: true,
    favMap: {}
  },

  onLoad: function () {
    console.log('📺 视频频道加载');
    var app = getApp();
    this.setData({ autoPlay: !!app.getSettings().autoPlay });
    this.syncFav();
  },

  onShow: function () {
    this.syncFav();
  },

  syncFav: function () {
    var map = {};
    getApp().getFavorites().forEach(function (a) { map['a' + a.id] = true; });
    this.setData({ favMap: map });
  },

  onSelectFilter: function (e) {
    var id = Number(e.currentTarget.dataset.id);
    this.setData({
      currentFilter: id,
      videos: ALL_VIDEOS[id] || ALL_VIDEOS[1],
      playingId: 0
    });
  },

  // 点击封面 → 该卡片切换为真实的 <video> 播放器
  onPlay: function (e) {
    var id = Number(e.currentTarget.dataset.id);
    this.setData({ playingId: this.data.playingId === id ? 0 : id });
    console.log('▶️ 播放视频 ' + id);
  },

  onAutoPlayChange: function (e) {
    getApp().saveSettings({ autoPlay: e.detail.value });
    this.setData({ autoPlay: e.detail.value });
    wx.showToast({ title: e.detail.value ? '非 Wi-Fi 也自动播放' : '已关闭自动播放', icon: 'none' });
  },

  onToggleFav: function (e) {
    var ds = e.currentTarget.dataset;
    var on = getApp().toggleFavorite({ id: Number(ds.id), title: ds.title, source: '视频' });
    this.syncFav();
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  onOpenArticle: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onSearchTap: function () {
    wx.navigateTo({ url: '/pages/search/search' });
  }
});
