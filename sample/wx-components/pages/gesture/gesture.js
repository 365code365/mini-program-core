const app = getApp();
const NAMES = ['POSSIBLE', 'BEGIN', 'ACTIVE', 'END', 'CANCELLED'];
Page({
  data: { x: 0, y: 0, showSheet: false, show: false, items: Array.from({ length: 20 }, (_, i) => `面板里的第 ${i + 1} 行`), log: [] },
  // worklet:ongesture：参数就是手势事件对象（字段在顶层）
  onPan(e) {
    if (e.state === 2) this.setData({ x: this.data.x + e.deltaX, y: this.data.y + e.deltaY });
    if (e.state !== 2) app.note(this, `pan ${NAMES[e.state]} vx=${Math.round(e.velocityX)} vy=${Math.round(e.velocityY)}`);
  },
  onTapG(e) { app.note(this, `tap-gesture state=${e.state}`); },
  onDouble(e) { this.setData({ x: 0, y: 0 }); app.note(this, `double-tap state=${e.state}：复位`); },
  onVDrag(e) { if (e.state === 1 || e.state === 3) app.note(this, `外层 vertical-drag ${NAMES[e.state]}（竖着拖它赢，pan 不动）`); },
  toggleSheet() { this.setData({ showSheet: !this.data.showSheet }); },
  onSize(e) { /* worklet:onsizeupdate {pixels, size} */ this.lastSize = e.size; },
  sheetTo(e) {
    this.selectComponent('#sheet') && this.createSelectorQuery().select('#sheet').node().exec((res) => {
      res[0] && res[0].node && res[0].node.scrollTo({ size: Number(e.currentTarget.dataset.s), duration: 300 });
    });
  },
  openPc() { this.setData({ show: true }); },
  closePc() { this.setData({ show: false }); },
  onPcEvent(e) { app.note(this, `page-container ${e.type}`); }
});
