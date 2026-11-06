const COLS = 16;
const CITIES = ['北京', '上海', '广州', '深圳', '杭州', '成都', '武汉', '西安', '南京', '重庆', '天津', '苏州'];
Page({
  data: {
    cols: Array.from({ length: COLS }, (_, c) => `${c + 1} 月`),
    rows: Array.from({ length: 40 }, (_, r) => ({
      id: r,
      name: `${CITIES[r % CITIES.length]} ${Math.floor(r / CITIES.length) + 1} 店`,
      cells: Array.from({ length: COLS }, (_, c) => ((r * 31 + c * 17) % 90) + 10)
    })),
    left: 0, top: 0, picked: '-'
  },
  // 横竖同时滚：bindscroll 的 detail 里 scrollLeft / scrollTop 都有
  onScroll(e) {
    const { scrollLeft, scrollTop } = e.detail;
    const left = Math.round(scrollLeft), top = Math.round(scrollTop);
    if (left !== this.data.left || top !== this.data.top) this.setData({ left, top });
  },
  onCell(e) {
    const { r, c } = e.currentTarget.dataset;
    const row = this.data.rows[r];
    this.setData({ picked: `${row.name} · ${this.data.cols[c]} = ${row.cells[c]}` });
  },
  // scroll-left / scroll-top 只在值变了时生效：手动滚走之后再点同一个按钮，换一个相邻的值
  toEnd() { const v = this.data.toLeft === 4000 ? 4001 : 4000; this.setData({ toLeft: v, toTop: v }); },
  toStart() { const v = this.data.toLeft === 0 ? 0.5 : 0; this.setData({ toLeft: v, toTop: v }); }
});
