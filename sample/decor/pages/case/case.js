const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

// 两处 Skyline 手势 + worklet：
// 1. 图集外包 double-tap-gesture-handler：双击收藏，爱心用 shared 值 + applyAnimatedStyle 做「弹出 → 停留 → 淡出」；
// 2. 装修前后对比外包 pan-gesture-handler：横向拖动分割线，shared 值驱动「装修前」那层的宽度与把手位置。
// 引擎里 worklet 函数在逻辑层同步执行（见 README），写法与微信一致，'worklet' 指令照写。
const { shared, timing, sequence, delay, Easing } = wx.worklet;

Page({
  data: { c: null, d: null, fav: false, cur: 0, splits: [], cmpW: 0, others: [] },

  onLoad(q) {
    const c = D.caseById(q.id) || D.CASES[0];
    const d = D.designer(c.designer);
    const splits = c.split.map((p, i) => ({ name: D.SPLIT_NAMES[i], percent: p, wan: (c.budget * p / 100).toFixed(1) }));
    const others = D.CASES.filter((x) => x.designer === c.designer && x.id !== c.id).slice(0, 2);
    this.setData({ c, d, splits, others, fav: store.get(store.K.favCases, []).indexOf(c.id) >= 0 });
    wx.setNavigationBarTitle({ title: c.style + ' · ' + c.area + '㎡' });
    this.setupWorklets();
  },

  setupWorklets() {
    const heart = shared(0);
    this._heart = heart;
    this.applyAnimatedStyle('.heart', () => {
      'worklet';
      return { opacity: Math.min(1, heart.value * 1.4), transform: 'scale(' + (0.4 + heart.value * 0.8) + ')' };
    });
    const split = shared(0);
    this._split = split;
    this.applyAnimatedStyle('.cmp-before', () => {
      'worklet';
      return { width: split.value + 'px' };
    });
    this.applyAnimatedStyle('.cmp-handle', () => {
      'worklet';
      return { left: (split.value - 16) + 'px' };
    });
    // 量出对比区宽度，分割线放中间
    setTimeout(() => {
      wx.createSelectorQuery().select('#cmp').boundingClientRect().exec((res) => {
        const r = res && res[0];
        if (!r) return;
        this._cmpW = r.width;
        this.setData({ cmpW: r.width });
        split.value = r.width / 2;
      });
    }, 50);
  },

  // ---------- 双击收藏 ----------
  onDoubleTap(e) {
    'worklet';
    if (e.state !== 3) return;
    this._heart.value = sequence(timing(1, { duration: 180, easing: Easing.out(Easing.back(2)) }), delay(380, timing(0, { duration: 260 })));
    if (!this.data.fav) this.setFav(true);
  },
  toggleFav() { this.setFav(!this.data.fav); },
  setFav(on) {
    const id = this.data.c.id;
    const list = store.get(store.K.favCases, []);
    if ((list.indexOf(id) >= 0) !== on) store.toggle(store.K.favCases, id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏，可在「我的」查看' : '已取消收藏', icon: 'none' });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  preview(e) { wx.previewImage({ urls: this.data.c.gallery, current: e.currentTarget.dataset.src }); },

  // ---------- 装修前后对比 ----------
  onComparePan(e) {
    'worklet';
    if (e.state !== 1 && e.state !== 2) return;
    const w = this._cmpW || 0;
    this._split.value = Math.max(0, Math.min(w, this._split.value + e.deltaX));
  },
  showSide(e) {
    const w = this._cmpW || 0;
    const to = { before: w, after: 0, half: w / 2 }[e.currentTarget.dataset.side];
    this._split.value = timing(to, { duration: 320 });
  },

  goDesigner() { wx.navigateTo({ url: '/pages/designer/designer?id=' + this.data.d.id }); },
  openCase(e) { wx.redirectTo({ url: '/pages/case/case?id=' + e.currentTarget.dataset.id }); },
  book() {
    const c = this.data.c;
    wx.navigateTo({ url: '/pages/booking/booking?case=' + c.id + '&designer=' + c.designer + '&area=' + c.area });
  },
  onShareAppMessage() { return { title: this.data.c.title, path: '/pages/case/case?id=' + this.data.c.id }; }
});
