// 宝贝详情：图集、价格成色、卖家、留言、TA 还在卖；底栏收藏 / 分享 / 聊一聊 / 我想要
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
Page({
  data: { it: null, cur: 0, fav: false, comments: [], cText: '', cFocus: false, others: [], order: null },
  onLoad(q) {
    this._id = q.id;
    wx.showShareMenu({ withShareTicket: true, menus: ['shareAppMessage', 'shareTimeline'] });
    this.load();
  },
  onShow() { if (this._loaded) this.load(); this._loaded = true; },
  load() {
    const it = store.findItem(this._id);
    if (!it) { this.setData({ it: null }); return; }
    const saved = store.get(store.K.comments, {})[it.id] || [];
    const others = store.allItems().filter(x => x.seller === it.seller && x.id !== it.id).slice(0, 6)
      .map(x => ({ id: x.id, cover: x.cover, price: x.price, title: x.title }));
    this.setData({
      it, fav: store.isFav(it.id), others, order: store.orderForItem(it.id) || null,
      comments: (D.SEED_COMMENTS[it.id] || []).concat(saved), onSaleCount: others.length + 1
    });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  preview(e) { wx.previewImage({ urls: this.data.it.imgs, current: this.data.it.imgs[e.currentTarget.dataset.i] }); },
  toggleFav() {
    const on = store.toggleFav(this.data.it.id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏，降价会提醒你' : '已取消收藏', icon: 'none' });
  },
  focusComment() { this.setData({ cFocus: true }); },
  onCInput(e) { this.setData({ cText: e.detail.value }); },
  onCBlur() { this.setData({ cFocus: false }); },
  sendComment() {
    const text = this.data.cText.trim();
    if (!text) { wx.showToast({ title: '写点什么再留言吧', icon: 'none' }); return; }
    if (text.length > 100) { wx.showToast({ title: '留言最多 100 字', icon: 'none' }); return; }
    const all = store.get(store.K.comments, {});
    const c = { user: D.ME.name, avatar: D.ME.avatar, text, time: '刚刚', mine: true };
    all[this.data.it.id] = (all[this.data.it.id] || []).concat([c]);
    store.set(store.K.comments, all);
    this.setData({ comments: this.data.comments.concat([c]), cText: '', cFocus: false });
    wx.showToast({ title: '留言成功', icon: 'success' });
  },
  goOther(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  chat() {
    const it = this.data.it;
    if (it.mine) { wx.showToast({ title: '这是你自己发布的宝贝', icon: 'none' }); return; }
    const c = store.convForItem(it.id);
    wx.navigateTo({ url: '/pages/chat/chat?id=' + c.id });
  },
  buy() {
    const it = this.data.it;
    if (this.data.order) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.order.id }); return; }
    if (it.mine) { wx.navigateTo({ url: '/pages/my-items/my-items' }); return; }
    if (it.sold) { wx.showToast({ title: '宝贝已经卖出了', icon: 'none' }); return; }
    wx.navigateTo({ url: '/pages/order/order?id=' + it.id });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  onShareAppMessage() {
    const it = this.data.it;
    if (!it) return { title: '闲转：同城闲置', path: '/pages/index/index' };
    return { title: '¥' + it.price + ' ' + it.title, path: '/pages/detail/detail?id=' + it.id, imageUrl: it.cover };
  },
  onShareTimeline() {
    const it = this.data.it;
    return it ? { title: '¥' + it.price + ' ' + it.title, query: 'id=' + it.id, imageUrl: it.cover } : { title: '闲转：同城闲置' };
  }
});
