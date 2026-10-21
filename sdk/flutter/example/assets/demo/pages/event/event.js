Page({
  data: {
    tapPath: '',
    pressInfo: '试试短按和长按',
    touching: false,
    fx: 0,
    fy: 0,
    touchInfo: '还没触摸',
    padTop: 0,
    padLeft: 0
  },
  path: [],
  onReady() {
    // 换成方块内坐标要减去方块位置。boundingClientRect 是视口坐标，加上当时的页面滚动距离
    // 才是页面坐标，之后拿 touch.pageY 去减，页面滚过也不会错位
    wx.createSelectorQuery()
      .select('.pad').boundingClientRect()
      .selectViewport().scrollOffset()
      .exec((res) => {
        const r = res[0];
        const vp = res[1] || { scrollTop: 0 };
        if (r) this.setData({ padTop: r.top + vp.scrollTop, padLeft: r.left });
      });
  },

  onCapture(e) {
    // 捕获阶段最先到：先清空上一次的路径
    this.path = ['capture:' + e.currentTarget.dataset.name];
  },
  onTap(e) {
    this.path.push(e.currentTarget.dataset.name);
    // 同一次点击的所有回调都在这一轮里跑完，最后一个设上去的就是完整路径
    this.setData({ tapPath: this.path.join(' → ') + '（target=' + e.target.dataset.name + '）' });
  },

  onShortTap(e) {
    this.setData({ pressInfo: 'tap  detail=' + JSON.stringify(e.detail) });
  },
  onLongPress(e) {
    const ds = e.currentTarget.dataset;
    this.setData({
      pressInfo: 'longpress  dataset=' + JSON.stringify(ds) + '\nid 的类型：' + typeof ds.id + '，on 的类型：' + typeof ds.on
    });
    wx.vibrateShort({ type: 'light' });
  },

  onTouchStart(e) {
    const t = e.touches[0];
    this.scrollAtStart = 0;
    this.moves = 0;
    this.update(t, 'touchstart');
  },
  onTouchMove(e) {
    this.moves++;
    this.update(e.touches[0], 'touchmove #' + this.moves);
  },
  onTouchEnd(e) {
    const t = e.changedTouches[0];
    this.setData({ touching: false, touchInfo: e.type + '  共 ' + (this.moves || 0) + ' 次 move，抬手于 (' + Math.round(t.clientX) + ', ' + Math.round(t.clientY) + ')' });
  },
  update(t, label) {
    if (!t) return;
    this.setData({
      touching: true,
      fx: t.clientX - this.data.padLeft,
      fy: (t.pageY === undefined ? t.clientY : t.pageY) - this.data.padTop,
      touchInfo: label + '  clientX=' + Math.round(t.clientX) + ' clientY=' + Math.round(t.clientY) + ' pageY=' + Math.round(t.pageY)
    });
  }
});
