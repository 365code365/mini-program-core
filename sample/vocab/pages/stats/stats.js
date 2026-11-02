// 学习统计：总量、近 7 / 30 天每日新学 + 复习的柱状图（view 画柱子）、各复习阶段的词数分布
const store = require('../../utils/store.js');
const D = require('../../utils/date.js');
const STAGES = ['第 1 轮', '第 2 轮', '第 3 轮', '第 4 轮', '第 5 轮', '已掌握'];

Page({
  data: { range: 7, bars: [], maxV: 1, sumLearn: 0, sumReview: 0, totals: {}, stages: [], avg: 0, best: null },
  onShow() { this.render(); },
  onRange(e) { this.setData({ range: +e.currentTarget.dataset.r }); this.render(); },
  render() {
    const n = this.data.range, logs = store.logs(), t = D.today();
    const bars = [];
    let sumLearn = 0, sumReview = 0, best = null;
    for (let i = n - 1; i >= 0; i--) {
      const ds = D.addDays(t, -i);
      const l = Object.assign({ learn: 0, review: 0, quiz: 0 }, logs[ds]);
      sumLearn += l.learn; sumReview += l.review;
      const v = l.learn + l.review;
      if (!best || v > best.v) best = { v, label: D.short(ds) };
      // 30 天时只给每 5 天标一次日期，避免挤在一起
      const showLb = n === 7 || i % 5 === 0;
      bars.push({ ds, learn: l.learn, review: l.review, v, lb: showLb ? (n === 7 ? D.weekday(ds).slice(1) : D.short(ds)) : '' });
    }
    const maxV = Math.max(4, ...bars.map((b) => b.v));
    bars.forEach((b) => {
      b.hl = Math.round(b.learn * 240 / maxV);
      b.hr = Math.round(b.review * 240 / maxV);
    });
    const p = store.progress(), ws = Object.keys(p);
    const cnt = [0, 0, 0, 0, 0, 0];
    ws.forEach((w) => { cnt[p[w].due ? Math.min(4, p[w].stage) : 5] += 1; });
    const stages = STAGES.map((name, i) => ({ name, n: cnt[i], pct: ws.length ? Math.round(cnt[i] * 100 / ws.length) : 0 }));
    this.setData({
      bars, maxV, sumLearn, sumReview, stages, best: best && best.v ? best : null,
      avg: Math.round((sumLearn + sumReview) * 10 / n) / 10,
      totals: { learned: ws.length, mastered: cnt[5], stars: store.stars().length, days: store.checkins().length }
    });
  }
});
