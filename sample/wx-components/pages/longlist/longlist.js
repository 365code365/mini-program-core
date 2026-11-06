const app = getApp();
const TAGS = ['数码', '家居', '美食', '出行', '运动', '阅读'];
function makeRows(from, n) {
  return Array.from({ length: n }, (_, k) => {
    const i = from + k;
    return { id: i, title: `第 ${i + 1} 条 · ${TAGS[i % TAGS.length]}`, sub: `长列表里改一行，只重排这一行（第 ${i + 1} 行的副标题）`, likes: (i * 37) % 100 };
  });
}
Page({
  data: { rows: makeRows(0, 300), counter: 0, cost: '-', hot: 149, log: [] },
  // 改一样东西，从 setData 到回调（这一帧画完）用了多久
  timed(patch, what) {
    const t0 = Date.now();
    this.setData(patch, () => {
      const ms = Date.now() - t0;
      this.setData({ cost: `${what}：${ms}ms` });
      app.note(this, `${what} ${ms}ms（共 ${this.data.rows.length} 行）`);
    });
  },
  likeOne() {
    const i = this.data.hot;
    this.timed({ [`rows[${i}].likes`]: this.data.rows[i].likes + 1 }, `第 ${i + 1} 行点赞`);
  },
  likeRow(e) {
    const i = e.currentTarget.dataset.i;
    this.timed({ hot: i, [`rows[${i}].likes`]: this.data.rows[i].likes + 1 }, `第 ${i + 1} 行点赞`);
  },
  bump() { this.timed({ counter: this.data.counter + 1 }, '顶上计数 +1'); },
  append() {
    const n = this.data.rows.length;
    this.timed({ rows: this.data.rows.concat(makeRows(n, 50)) }, '追加 50 行');
  }
});
