const store = require('../../utils/store.js');
Page({
  data: { list: [], joinedCount: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.load();
  },
  load() {
    const j = store.joined(); const all = store.posts();
    const list = store.CIRCLES.map((c) => ({
      id: c.id, name: c.name, desc: c.desc, color: c.color, bg: store.tint(c.color, 0.12), initial: c.name.slice(0, 1),
      joined: j.indexOf(c.id) >= 0, members: store.fmtNum(c.members + (j.indexOf(c.id) >= 0 ? 1 : 0)),
      posts: all.filter((p) => p.circle === c.id).length
    }));
    list.sort((a, b) => b.joined - a.joined);
    this.setData({ list, joinedCount: j.length });
  },
  toggle(e) {
    const id = e.currentTarget.dataset.id;
    let j = store.joined();
    const on = j.indexOf(id) >= 0;
    if (on) {
      wx.showModal({ title: '退出圈子', content: '退出后将不再在「已加入」里看到该圈子的帖子', confirmText: '退出', success: (r) => {
        if (!r.confirm) return;
        wx.setStorageSync('forum_joined', store.joined().filter((x) => x !== id));
        this.load();
      } });
      return;
    }
    j = j.concat([id]);
    wx.setStorageSync('forum_joined', j);
    this.load();
    wx.showToast({ title: '已加入' + store.circle(id).name + '圈', icon: 'none' });
  },
  open(e) { wx.navigateTo({ url: '/pages/circle/circle?id=' + e.currentTarget.dataset.id }); },
  // 回首页并切到这个圈子的 tab：switchTab 不能带参数，经 storage 传
  browse(e) {
    wx.setStorageSync('forum_go_circle', e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/feed/feed' });
  }
});
