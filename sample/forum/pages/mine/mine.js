const store = require('../../utils/store.js');
const social = require('../../utils/social.js');
Page({
  data: { me: store.ME, initial: store.ME.name.slice(0, 1), seg: 0, list: [], stats: [], flash: '', empty: false, joinedText: '' },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.load();
  },
  load() {
    const all = store.posts(); const likes = store.likes(); const favs = store.favs();
    const mine = all.filter((p) => p.mine);
    const src = this.data.seg === 0 ? mine : all.filter((p) => favs.indexOf(p.id) >= 0);
    const list = src.map((p) => store.card(p, likes, favs));
    const got = mine.reduce((a, p) => a + p.likes + (likes.indexOf(p.id) >= 0 ? 1 : 0), 0);
    const j = store.joined();
    this.setData({
      list, empty: list.length === 0,
      joinedText: '已加入 ' + j.length + ' 个圈子：' + j.map((id) => store.circle(id).name).join('、'),
      stats: [{ k: 'p', v: mine.length, l: '帖子' }, { k: 'l', v: got, l: '获赞' }, { k: 'f', v: favs.length, l: '收藏' }, { k: 'c', v: j.length, l: '圈子' }]
    });
  },
  onSeg(e) { this.setData({ seg: Number(e.currentTarget.dataset.i) }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/post/post?id=' + e.detail.id }); },
  like(e) { social.toggle(this, 'list', 'like', e.detail.id); },
  fav(e) {
    social.toggle(this, 'list', 'fav', e.detail.id);
    // 收藏列表里取消收藏：等确认写入后再移出列表
    if (this.data.seg === 1) setTimeout(() => this.load(), 650);
  },
  goCircle() {},
  edit(e) {
    const id = e.currentTarget.dataset.id;
    wx.navigateTo({
      url: '/pages/compose/compose?id=' + id,
      events: {
        updated: (r) => {
          this.load();
          this.setData({ flash: id });
          wx.showToast({ title: '已更新「' + r.title.slice(0, 8) + '」', icon: 'none' });
          clearTimeout(this.ft);
          this.ft = setTimeout(() => this.setData({ flash: '' }), 1500);
        }
      }
    });
  },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    const p = store.posts().find((x) => x.id === id);
    wx.showModal({
      title: '删除帖子', content: '确定删除「' + p.title + '」吗？删除后评论也会一并清除，无法恢复。', confirmText: '删除', confirmColor: '#F53F3F',
      success: (r) => {
        if (!r.confirm) return;
        store.savePosts(store.posts().filter((x) => x.id !== id));
        wx.removeStorageSync('forum_cmt_' + id);
        wx.setStorageSync('forum_favs', store.favs().filter((x) => x !== id));
        this.load();
        wx.showToast({ title: '已删除', icon: 'none' });
      }
    });
  },
  compose() { wx.navigateTo({ url: '/pages/compose/compose' }); },
  reset() {
    wx.showModal({ title: '恢复演示数据', content: '帖子、点赞、收藏、评论与草稿将恢复初始状态', success: (r) => {
      if (!r.confirm) return;
      wx.clearStorageSync();
      store.ensure();
      this.load();
      wx.showToast({ title: '已恢复', icon: 'none' });
    } });
  }
});
