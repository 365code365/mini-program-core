const app = getApp();
const COLORS = ['#ffd6d6', '#ffe9c7', '#fff6bf', '#d9f7d6', '#d6ecff', '#e6dcff'];
Page({
  data: {
    list: Array.from({ length: 200 }, (_, i) => ({ id: i, name: `第 ${i + 1} 行` })),
    cards: Array.from({ length: 24 }, (_, i) => ({ id: i, h: 120 + (i * 53) % 140, c: COLORS[i % COLORS.length] })),
    built: 0, disposed: 0, log: []
  },
  onLoad() { this.counts = { built: 0, disposed: 0 }; },
  // 滚动时每进出一项就来一次事件：计数先攒在实例上，300ms 内最多 setData 一次。
  // 每个事件都 setData 会让整页（200 行 + 两组网格）每帧重建一次布局，真机上滑起来就顿
  flush() {
    if (this.timer) return;
    this.timer = setTimeout(() => {
      this.timer = null;
      this.setData({ built: this.counts.built, disposed: this.counts.disposed });
    }, 300);
  },
  onBuild(e) {
    // detail.index 是 number
    this.counts.built += 1;
    this.flush();
    if (e.detail.index % 20 === 0) app.note(this, `itembuild index=${e.detail.index} (${typeof e.detail.index})`);
  },
  onDispose() { this.counts.disposed += 1; this.flush(); },
  onUnload() { if (this.timer) clearTimeout(this.timer); }
});
