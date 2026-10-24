const G = require('../../utils/game.js');
const QS = require('../../utils/questions.js');

Page({
  data: { r: {}, shown: 0, accText: '', timeText: '', stars: [0, 0, 0], tier: {}, tierHint: '', tierUp: false, wrongs: [], energy: 0, recordText: '', bestText: '' },

  onLoad() {
    // 没打过就直接打开时给一份示例战绩，页面不至于空
    const r = G.get('lastResult', null) || { score: 1460, correct: 8, answered: 10, skipped: 0, total: 10, maxCombo: 5, seconds: 46.3, newRecord: true, prevBest: 1210, wrongIds: ['q3', 'q15'], totalBefore: 3260 };
    const acc = r.answered ? r.correct / r.answered : 0;
    const before = G.tier(r.totalBefore);
    const now = G.tier(r.totalBefore + r.score);
    this.setData({
      r,
      accText: Math.round(acc * 100) + '%',
      timeText: r.seconds + ' 秒',
      stars: [acc >= 0.4 ? 1 : 0, acc >= 0.7 ? 1 : 0, acc >= 0.9 ? 1 : 0],
      tier: now,
      tierHint: now.next ? '距' + now.next + '还差 ' + now.need + ' 分' : '已是最高段位',
      tierUp: now.idx > before.idx,
      recordText: r.prevBest ? '超过之前最好成绩 ' + r.prevBest + ' 分' : '首次完成挑战，创下个人纪录',
      bestText: '历史最好 ' + Math.max(r.prevBest, r.score) + ' 分，再接再厉',
      wrongs: r.wrongIds.map(id => QS.find(id)).filter(Boolean).map(q => ({ id: q.id, q: q.q })),
      energy: G.energy().energy
    });
    this.countUp(r.score);
  },
  onUnload() { clearInterval(this.cu); },

  // 分数滚动上涨：约 0.8 秒滚到终值
  countUp(target) {
    let n = 0;
    const step = Math.max(1, Math.ceil(target / 26));
    clearInterval(this.cu);
    this.cu = setInterval(() => {
      n = Math.min(target, n + step);
      this.setData({ shown: n });
      if (n >= target) clearInterval(this.cu);
    }, 30);
  },

  again() {
    if (!G.useEnergy()) return wx.showToast({ title: '体力不足，回首页补充', icon: 'none' });
    wx.redirectTo({ url: '/pages/quiz/quiz' });
  },
  home() { wx.switchTab({ url: '/pages/index/index' }); },
  rank() { wx.switchTab({ url: '/pages/rank/rank' }); },
  goBook() { wx.switchTab({ url: '/pages/wrongbook/wrongbook' }); },
  openWrong(e) { wx.navigateTo({ url: '/pages/wrong-detail/wrong-detail?id=' + e.currentTarget.dataset.id }); }
});
