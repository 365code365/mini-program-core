const { live, farmers, goodsById } = require('../../utils/data.js');
const store = require('../../utils/store.js');
const ticker = require('../../utils/ticker.js');

// live-player 状态码（官方 bindstatechange）：2001 已连接、2004 开始播放、2103 重连、-2301 多次重连失败、3005 读数据失败
const STATE = { 2001: '已连接服务器', 2002: '已连接，等待画面', 2004: '直播中', 2007: '缓冲中…', 2103: '网络断连，正在重连', 2105: '当前视频播放卡顿', '-2301': '直播信号中断' };
const WAIT_MS = 3000;

function wan(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : String(n); }

Page({
  data: {
    mode: 'live', src: live.src, replay: live.replay, cover: live.cover, title: live.title, muted: false, replaying: false, fallbackText: '',
    stateText: '连接中…', farmer: farmers[live.farmer], viewersText: '', likesText: '', followed: false, bag: [], comments: [], draft: '', myLikes: 0, schedule: []
  },

  onLoad() {
    this.viewers = live.viewers;
    this.likes = 86420;
    this.cmIdx = 0;
    this.setData({
      bag: live.goods.map((id) => { const g = goodsById(id); return { id, cover: g.cover, short: g.name.split(' ')[0], groupPrice: g.groupPrice }; }),
      comments: live.comments.slice(0, 4).map((c, i) => ({ id: 'c' + i, name: c[0], text: c[1] })),
      followed: (wx.getStorageSync('farm_follow') || []).indexOf(live.farmer) >= 0
    });
    this.cmIdx = 4;
    this.refreshCount();
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1, badge: store.cartCount() });
    this.loadSchedule();
    this.startLive();
    ticker.attach(this, (now) => this.tick(now));
  },
  onHide() { this.leave(); },
  onUnload() { this.leave(); },
  leave() {
    clearTimeout(this._wait);
    if (this.ctx && this.data.mode === 'live') this.ctx.stop({ fail() {} });
    ticker.detach(this);
  },

  startLive() {
    this.playing = false;
    this.setData({ mode: 'live', replaying: false, stateText: '连接中…' });
    this.ctx = wx.createLivePlayerContext('farmLive', this);
    this.ctx.play({ fail: () => {} });
    clearTimeout(this._wait);
    this._wait = setTimeout(() => { if (!this.playing) this.fallback('当前环境无法播放直播流'); }, WAIT_MS);
  },
  onLiveState(e) {
    const code = e.detail.code;
    if (code === 2004) { this.playing = true; clearTimeout(this._wait); }
    this.setData({ stateText: STATE[code] || '直播中' });
    if (code === -2301 || code === 3005) this.fallback('直播信号中断');
  },
  onLiveError(e) { this.fallback('直播加载失败：' + (e.detail && e.detail.errMsg ? e.detail.errMsg : '未知错误')); },
  fallback(text) {
    clearTimeout(this._wait);
    this.setData({ mode: 'fallback', fallbackText: text, stateText: '回放可看' });
  },
  watchReplay() { this.setData({ replaying: true, stateText: '今日回放' }); },
  onReplayEnd() { this.setData({ stateText: '回放结束' }); },
  retryLive() { wx.showToast({ title: '正在重新连接', icon: 'loading', duration: 1000 }); this.startLive(); },
  toggleMute() {
    const m = !this.data.muted;
    this.setData({ muted: m });
    if (this.ctx) this.ctx.mute({ fail() {} });
  },

  // 观看人数小幅波动、隔几秒滚一条评论（与倒计时共用一个 ticker）
  tick(now) {
    this.viewers += Math.round(Math.sin(now / 7000) * 20 + 6);
    this.likes += 3 + (now % 7);
    const patch = {};
    if (Math.floor(now / 1000) % 3 === 0) {
      const c = live.comments[this.cmIdx % live.comments.length];
      this.cmIdx += 1;
      patch.comments = this.data.comments.concat({ id: 'c' + this.cmIdx, name: c[0], text: c[1] }).slice(-6);
    }
    this.setData(patch);
    this.refreshCount();
  },
  refreshCount() { this.setData({ viewersText: wan(this.viewers), likesText: wan(this.likes + this.data.myLikes) }); },

  onDraft(e) { this.setData({ draft: e.detail.value }); },
  send() {
    const t = this.data.draft.trim();
    if (!t) { wx.showToast({ title: '先写点什么吧', icon: 'none' }); return; }
    this.setData({ comments: this.data.comments.concat({ id: 'me' + Date.now(), name: '我', text: t }).slice(-6), draft: '' });
  },
  like() { wx.vibrateShort({ type: 'light' }); this.setData({ myLikes: this.data.myLikes + 1 }); this.refreshCount(); },
  toggleFollow() {
    const list = wx.getStorageSync('farm_follow') || [];
    const i = list.indexOf(live.farmer);
    if (i >= 0) list.splice(i, 1); else list.push(live.farmer);
    wx.setStorageSync('farm_follow', list);
    this.setData({ followed: i < 0 });
    wx.showToast({ title: i < 0 ? '已关注' : '已取消关注', icon: i < 0 ? 'success' : 'none' });
  },
  loadSchedule() {
    const on = store.reminders();
    this.setData({ schedule: live.schedule.map((s) => Object.assign({}, s, { farmerName: farmers[s.farmer].name, on: on.indexOf(s.id) >= 0 })) });
  },
  toggleRemind(e) {
    const id = e.currentTarget.dataset.id;
    const on = store.toggleRemind(id);
    this.loadSchedule();
    if (on) {
      // 开播提醒走订阅消息；拿不到授权时只在本地记一笔，进小程序时提醒
      wx.requestSubscribeMessage({
        tmplIds: ['Tz5lFarmLiveRemind01'],
        complete: () => wx.showToast({ title: '已预约，开播前提醒你', icon: 'success' })
      });
    } else wx.showToast({ title: '已取消预约', icon: 'none' });
  },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onShareAppMessage() { return { title: live.title, path: '/pages/live/live', imageUrl: live.cover }; }
});
