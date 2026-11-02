// 首页：分类宫格 + 双列瀑布流推荐。
// 曝光统计与图片渐显都靠 IntersectionObserver（视口下方多算 100px，图片进屏前就开始淡入）；
// 每批新卡片 setData 之后用 wx.nextTick 再挂观察者，保证选择器能选到刚渲染出来的节点
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
const PAGE = 6;
const FAIL = '/images/img-fail.jpg';
Page({
  data: {
    cats: D.CATS,
    tabs: [{ k: 'rec', t: '猜你喜欢' }, { k: 'new', t: '最新发布' }, { k: 'free', t: '包邮' }],
    tab: 'rec', left: [], right: [], loading: false, noMore: false, showTop: false, exposed: 0, total: 0, fail: FAIL
  },
  onLoad() { this._seq = 0; this.reload(); },
  onShow() {
    store.syncTab(this, 0);
    if (this._shown && store.get('xz_feed_dirty', false)) { store.set('xz_feed_dirty', false); this.reload(); }
    this._shown = true;
  },
  onUnload() { this._ob && this._ob.disconnect(); },
  source() {
    let list = store.allItems().filter(it => !it.sold);
    if (this.data.tab === 'new') list.sort((a, b) => (a.created ? 0 : a.days + 1) - (b.created ? 0 : b.days + 1));
    if (this.data.tab === 'free') list = list.filter(it => it.free);
    if (this.data.tab === 'rec' && list.length) {
      const n = (this._seq * 5) % list.length;   // 下拉刷新换一批：把列表轮转一段
      list = list.slice(n).concat(list.slice(0, n));
    }
    return list;
  },
  reload() {
    this._src = this.source();
    this._page = 0; this._lh = 0; this._rh = 0; this._seen = {};
    this._left = []; this._right = [];
    this.setData({ left: [], right: [], noMore: false, total: this._src.length });
    this.loadMore();
  },
  loadMore() {
    const start = this._page * PAGE;
    this._src.slice(start, start + PAGE).forEach(it => {
      const card = { id: it.id, title: it.title, price: it.price, cover: it.cover, cond: it.cond, free: it.free, wants: it.wants,
        name: it.sellerInfo.name, avatar: it.sellerInfo.avatar, mine: it.mine, shown: false, err: false };
      // 估算列高：图片高宽比 + 文字区约 0.62 个卡宽，往矮的那列放
      const h = (it.ratio || 1) + 0.62;
      if (this._lh <= this._rh) { this._left.push(card); this._lh += h; } else { this._right.push(card); this._rh += h; }
    });
    this._page++;
    this.setData({ left: this._left, right: this._right, loading: false, noMore: start + PAGE >= this._src.length });
    wx.nextTick(() => this.observe());
  },
  observe() {
    this._ob && this._ob.disconnect();
    this._ob = wx.createIntersectionObserver(this, { observeAll: true }).relativeToViewport({ bottom: 100 });
    this._ob.observe('.fcard', res => {
      if (res.intersectionRatio <= 0) return;
      const { id, col } = res.dataset;
      if (this._seen[id]) return;
      this._seen[id] = true;
      store.addExpose(id);
      const arr = col === 'l' ? this._left : this._right;
      const idx = arr.findIndex(c => c.id === id);
      if (idx < 0) return;
      arr[idx].shown = true;
      this.setData({ [(col === 'l' ? 'left' : 'right') + '[' + idx + '].shown']: true, exposed: this.data.exposed + 1 });
    });
  },
  onTab(e) {
    const k = e.currentTarget.dataset.k;
    if (k === this.data.tab) return;
    this.setData({ tab: k });
    this.reload();
  },
  onImgErr(e) {
    const { id, col } = e.currentTarget.dataset;
    const arr = col === 'l' ? this._left : this._right;
    const idx = arr.findIndex(c => c.id === id);
    if (idx >= 0 && !arr[idx].err) { arr[idx].err = true; this.setData({ [(col === 'l' ? 'left' : 'right') + '[' + idx + '].err']: true }); }
  },
  onCard(e) {
    if (Date.now() - (this._lp || 0) < 800) return;   // 长按松手后不再当成点击
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },
  onLongPress(e) {
    this._lp = Date.now();
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['不感兴趣', '举报该宝贝'],
      success: r => { r.tapIndex === 0 ? this.dislike(id) : this.reportItem(id); }
    });
  },
  dislike(id) {
    store.hide(id);
    this._left = this._left.filter(c => c.id !== id);
    this._right = this._right.filter(c => c.id !== id);
    this._src = this._src.filter(c => c.id !== id);
    this.setData({ left: this._left, right: this._right, total: this._src.length });
    wx.showToast({ title: '将减少此类推荐', icon: 'none' });
  },
  reportItem(id) {
    const reasons = ['图文不符 / 虚假宝贝', '疑似违禁品', '价格欺诈', '引导站外交易'];
    wx.showActionSheet({
      itemList: reasons,
      success: r => {
        store.report(id, reasons[r.tapIndex]);
        wx.showToast({ title: '举报已提交，24 小时内处理', icon: 'none' });
      }
    });
  },
  goSearch() { wx.navigateTo({ url: '/pages/search/search' }); },
  goCat(e) {
    const c = this.data.cats[e.currentTarget.dataset.i];
    wx.navigateTo({ url: '/pages/search/search?cat=' + c.key + '&name=' + c.name });
  },
  toTop() { wx.pageScrollTo({ scrollTop: 0, duration: 300 }); },
  onPageScroll(e) {
    const show = e.scrollTop > 520;
    if (show !== this.data.showTop) this.setData({ showTop: show });
  },
  onReachBottom() {
    if (this.data.loading || this.data.noMore) return;
    this.setData({ loading: true });
    setTimeout(() => this.loadMore(), 450);
  },
  onPullDownRefresh() {
    this._seq++;
    setTimeout(() => {
      this.reload();
      wx.stopPullDownRefresh();
      wx.showToast({ title: '已为你换一批', icon: 'none' });
    }, 500);
  },
  onShareAppMessage() { return { title: '闲转：同城闲置，捡漏就在今天', path: '/pages/index/index' }; },
  onShareTimeline() { return { title: '闲转：同城二手闲置买卖' }; }
});
