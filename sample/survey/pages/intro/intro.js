const data = require('../../utils/data.js');
const TYPE_TEXT = { single: '单选', multi: '多选', rate: '评分', text: '填空' };
// 问卷介绍：作答前先看清题量、用时、截止；已参与 / 已截止 / 有草稿时给不同入口
Page({
  data: { sv: null, ended: false, done: false, draftStep: 0, hasDraft: false, fav: false, minutes: 1, typesText: '', requiredCount: 0, joinedText: '' },
  onLoad(q) {
    this.id = q.id || 'q1';
  },
  onShow() {
    const sv = data.getSurvey(this.id);
    const done = !!data.getAnswers()[sv.id];
    const draft = data.getDraft(sv.id);
    const count = {};
    sv.questions.forEach(x => { count[x.type] = (count[x.type] || 0) + 1; });
    wx.setNavigationBarTitle({ title: sv.kind === '投票' ? '投票详情' : '问卷详情' });
    this.setData({
      sv,
      ended: data.isEnded(sv),
      done,
      hasDraft: !!draft,
      draftStep: draft ? draft.step + 1 : 0,
      fav: data.getFavs().indexOf(sv.id) >= 0,
      minutes: data.minutes(sv),
      typesText: Object.keys(count).map(k => TYPE_TEXT[k] + ' ' + count[k] + ' 题').join(' · '),
      requiredCount: sv.questions.filter(x => x.required).length,
      joinedText: data.fmtNum(sv.joined + (done ? 1 : 0))
    });
  },
  onStart() {
    if (this.data.ended) { wx.showToast({ title: '已截止，不能再作答', icon: 'none' }); return; }
    wx.navigateTo({ url: '/pages/answer/answer?id=' + this.data.sv.id });
  },
  // 重新作答：丢掉草稿从第一题开始，提交后覆盖上次的答案
  onRestart() {
    if (this.data.ended) { wx.showToast({ title: '已截止，不能再作答', icon: 'none' }); return; }
    wx.showModal({
      title: '重新作答',
      content: '提交后会覆盖你上次的答案',
      confirmText: '开始',
      confirmColor: '#6C5CE7',
      success: (r) => {
        if (!r.confirm) return;
        data.saveDraft(this.data.sv.id, null);
        wx.navigateTo({ url: '/pages/answer/answer?id=' + this.data.sv.id });
      }
    });
  },
  onResult() { wx.navigateTo({ url: '/pages/result/result?id=' + this.data.sv.id }); },
  onFav() {
    const on = data.toggleFav(this.data.sv.id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏，可在「我的」里找到' : '已取消收藏', icon: 'none' });
  },
  onShare() { wx.navigateTo({ url: '/pages/poster/poster?id=' + this.data.sv.id }); },
  onShareAppMessage() {
    return { title: this.data.sv.title, path: '/pages/intro/intro?id=' + this.data.sv.id };
  }
});
