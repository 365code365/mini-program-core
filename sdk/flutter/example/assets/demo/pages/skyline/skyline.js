// Skyline 组件与动画：grid-view、movable-view、手势组件 + worklet、draggable-sheet、this.animate
const COLORS = ['#07c160', '#10aeff', '#fa9d3b', '#576b95', '#e64340', '#8a5cf5'];
const HEIGHTS = [80, 140, 100, 60, 120, 90, 150, 70, 110];

Page({
  data: {
    gridType: 'masonry',
    cols: 2,
    cells: HEIGHTS.map((h, i) => ({ id: i + 1, h, c: COLORS[i % COLORS.length] })),
    mx: 20,
    my: 20,
    moveText: '还没拖',
    gestureText: '在灰色区域里拖、双击、长按',
    sheetPct: 40,
    animText: '点按钮看效果'
  },
  onReady() {
    const wk = wx.worklet;
    // 进度条：宽度跟着共享值（0~1）走
    this.progress = wk.shared(0);
    this.applyAnimatedStyle('#bar', () => ({ width: (this.progress.value * 100).toFixed(1) + '%' }));
    // 圆点：跟着 pan 手势的位移走
    this.dx = wk.shared(0);
    this.dy = wk.shared(0);
    this.applyAnimatedStyle('.pan-dot', () => ({ translateX: this.dx.value, translateY: this.dy.value }));
  },

  // ── grid-view ──
  setGrid(e) { this.setData({ gridType: e.currentTarget.dataset.t }); },
  nextCols() { this.setData({ cols: this.data.cols >= 4 ? 2 : this.data.cols + 1 }); },

  // ── movable-view ──
  onMove(e) {
    const d = e.detail;
    this.setData({ moveText: '圆 x=' + Math.round(d.x) + ' y=' + Math.round(d.y) + '  source=' + (d.source || "''") });
  },
  onMoveH(e) { this.setData({ moveText: '方块 x=' + Math.round(e.detail.x) + '  source=' + (e.detail.source || "''") }); },
  center() {
    wx.createSelectorQuery().select('#area').boundingClientRect((r) => {
      if (r) this.setData({ mx: Math.round((r.width - 64) / 2), my: Math.round((r.height - 64) / 2) });
    }).exec();
  },
  corner() {
    // 与微信一样：x / y 的值没变时 setData 不会挪动（拖过之后再回 0 也一样），所以在 0 与 0.01 之间换着给
    const z = this.data.mx === 0 ? 0.01 : 0;
    this.setData({ mx: z, my: z });
  },

  // ── 手势组件：state 1 BEGIN / 2 ACTIVE / 3 END / 4 CANCELLED ──
  onPan(e) {
    const wk = wx.worklet;
    if (e.state === 1) {
      this.base = { x: this.dx.value, y: this.dy.value };
    } else if (e.state === 2 && this.base) {
      this.dx.value = this.base.x + e.translationX;
      this.dy.value = this.base.y + e.translationY;
    } else if (e.state === 3 || e.state === 4) {
      this.dx.value = wk.spring(0, { stiffness: 180, damping: 14 });
      this.dy.value = wk.spring(0, { stiffness: 180, damping: 14 });
      this.setData({ gestureText: 'pan 结束：位移 ' + Math.round(e.translationX) + ', ' + Math.round(e.translationY) + '，速度 ' + Math.round(e.velocityX) + ' px/s' });
    }
  },
  onDouble(e) { if (e.state === 3) this.setData({ gestureText: '双击了（double-tap-gesture-handler）' }); },
  onLong(e) {
    if (e.state === 1) this.setData({ gestureText: '长按开始（BEGIN）' });
    else if (e.state === 3) this.setData({ gestureText: '长按松手（END）' });
  },

  // ── draggable-sheet ──
  onSheet(e) { this.setData({ sheetPct: Math.round(e.detail.size * 100) }); },
  sheetTo(e) {
    const sheet = this.selectComponent('#sheet');
    if (sheet) sheet.scrollTo({ size: Number(e.currentTarget.dataset.s), animated: true });
  },

  // ── this.animate / worklet ──
  play() {
    this.setData({ animText: 'this.animate 进行中…' });
    this.animate('#box', [
      { opacity: 1, rotate: 0, scale: [1, 1], backgroundColor: '#07c160' },
      { opacity: 0.6, rotate: 90, scale: [1.3, 1.3], backgroundColor: '#1677ff', ease: 'ease-out' },
      { opacity: 1, rotate: 180, scale: [1, 1], backgroundColor: '#fa9d3b' }
    ], 800, () => this.setData({ animText: 'this.animate 结束（最终样式保留，clearAnimation 才清）' }));
    this.progress.value = wx.worklet.timing(1, { duration: 800 });
  },
  spring() {
    const wk = wx.worklet;
    this.progress.value = wk.sequence(
      wk.timing(0, { duration: 1 }),
      wk.spring(1, { stiffness: 120, damping: 8 }, (finished) => this.setData({ animText: 'spring 落定：' + finished }))
    );
  },
  reset() {
    this.clearAnimation('#box', () => this.setData({ animText: '已清掉 #box 的动画样式' }));
    wx.worklet.cancelAnimation(this.progress);
    this.progress.value = 0;
  }
});
