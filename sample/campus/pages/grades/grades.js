// 成绩：multiSelector 选学年 + 学期，按学分加权算 GPA（4.0 制）与平均分；等级文字用 WXS 在视图层算
const D = require('../../utils/data.js');
const YEARS = ['2025-2026', '2026-2027'];
const TERMS = ['第一学期', '第二学期'];
Page({
  data: { range: [YEARS, TERMS], idx: [1, 0], list: [], gpa: '0.00', avg: '0.0', credits: 0, failed: 0 },
  onLoad() { this.calc(); },
  onPick(e) { this.setData({ idx: e.detail.value.map(Number) }); this.calc(); },
  onColumn() {},
  calc() {
    const [y, t] = this.data.idx;
    const list = (D.GRADES[YEARS[y] + '|' + (t + 1)] || []).map(g => Object.assign({}, g, { gp: D.gpaOf(g.score).toFixed(1), lv: D.levelOf(g.score), lvc: D.levelClassOf(g.score) }));
    const credits = list.reduce((s, g) => s + g.credit, 0);
    const gpa = credits ? list.reduce((s, g) => s + D.gpaOf(g.score) * g.credit, 0) / credits : 0;
    const avg = credits ? list.reduce((s, g) => s + g.score * g.credit, 0) / credits : 0;
    this.setData({ list, credits, gpa: gpa.toFixed(2), avg: avg.toFixed(1), failed: list.filter(g => g.score < 60).length });
  },
  rule() { wx.showModal({ title: '绩点换算', content: '90-100：4.0　85-89：3.7　82-84：3.3\n78-81：3.0　75-77：2.7　72-74：2.3\n68-71：2.0　64-67：1.5　60-63：1.0\n平均绩点 = Σ(课程绩点 × 学分) / Σ学分', showCancel: false }); }
});
