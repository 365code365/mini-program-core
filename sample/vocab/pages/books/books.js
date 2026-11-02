// 选词书：封面轮播（左右滑），滑动动画结束（bindanimationfinish）再刷新下方的词书详情，避免滑动途中频繁 setData
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');

Page({
  data: { list: [], cur: 0, using: 'cet4', info: null },
  onLoad() {
    const using = store.settings().book;
    const list = books.BOOKS.map((b) => {
      const total = books.words(b.id).length, learned = store.learnedOf(b.id).length;
      return Object.assign({}, b, { total, learned, pct: Math.round(learned * 100 / total) });
    });
    const cur = Math.max(0, list.findIndex((b) => b.id === using));
    this.setData({ list, cur, using, info: list[cur] });
  },
  onSwiperDone(e) {
    const i = e.detail.current;
    if (i === this.data.cur && this.data.info === this.data.list[i]) return;
    this.setData({ cur: i, info: this.data.list[i] });
  },
  onPick(e) { const i = e.currentTarget.dataset.i; this.setData({ cur: i, info: this.data.list[i] }); },
  onUse() {
    const b = this.data.info;
    if (b.id === this.data.using) { wx.navigateBack(); return; }
    wx.showModal({
      title: '切换到《' + b.name + '》？',
      content: '之后每天的新词从这本书里出；原词书的进度会保留，已学的词照常复习。',
      confirmText: '切换',
      success: (r) => {
        if (!r.confirm) return;
        store.saveSettings({ book: b.id });
        this.setData({ using: b.id });
        wx.showToast({ title: '已切换', icon: 'success' });
        setTimeout(() => wx.navigateBack({ fail: () => wx.switchTab({ url: '/pages/index/index' }) }), 600);
      }
    });
  }
});
