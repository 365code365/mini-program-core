// 求职偏好：期望职位 / 城市 / 薪资 / 求职状态 / 工作性质。首页的推荐排序与城市跟着它走
const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
const SALARY = ['不限', '8-12K', '12-20K', '20-30K', '30-50K', '50K 以上'];
Page({
  data: {
    intents: data.INTENTS, cities: data.CITIES, salaries: SALARY, statuses: S.JOB_STATUS, types: S.JOB_TYPES,
    v: { intent: '', cityIdx: 0, salary: 0, status: 0, type: 0 }, iIdx: 0, dirty: false
  },
  onLoad() {
    const v = S.getIntent();
    this.setData({ v, iIdx: Math.max(0, data.INTENTS.indexOf(v.intent)) });
  },
  onIntent(e) {
    const i = Number(e.detail.value);
    this.setData({ iIdx: i, 'v.intent': data.INTENTS[i], dirty: true });
  },
  onPick(e) {
    this.setData({ ['v.' + e.currentTarget.dataset.k]: Number(e.detail.value), dirty: true });
  },
  onType(e) {
    this.setData({ 'v.type': Number(e.currentTarget.dataset.i), dirty: true });
  },
  onSave() {
    S.saveIntent(this.data.v);
    // 简历里的期望职位跟着改，HR 看到的是同一个
    const r = S.getResume();
    r.position = this.data.v.intent;
    S.saveResume(r);
    this.setData({ dirty: false });
    wx.showToast({ title: '已保存', icon: 'success' });
  }
});
