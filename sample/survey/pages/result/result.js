const data = require('../../utils/data.js');

const TYPE_TEXT = { single: '单选', multi: '多选', rate: '评分', text: '填空' };

Page({
  data: { sv: null, fresh: false, stats: [], totalText: '' },
  onLoad(q) {
    const sv = data.getSurvey(q.id || 'q1');
    const mine = data.getAnswers()[sv.id] || {};
    const answered = Object.keys(mine).length > 0;
    const total = sv.joined + (answered ? 1 : 0);
    wx.setNavigationBarTitle({ title: q.fresh ? '提交成功' : '统计结果' });
    const stats = sv.questions.map(qq => this.stat(qq, mine[qq.id], total));
    this.setData({ sv, fresh: !!q.fresh, stats, totalText: String(total).replace(/\B(?=(\d{3})+(?!\d))/g, ',') });
  },
  stat(q, my, total) {
    const base = { id: q.id, title: q.title, typeText: TYPE_TEXT[q.type], n: total };
    const out = this.stat2(q, my, total, base);
    out.n = String(out.n).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
    return out;
  },
  stat2(q, my, total, base) {
    if (q.type === 'single' || q.type === 'multi') {
      const picked = q.type === 'single' ? (typeof my === 'number' ? [my] : []) : (my || []);
      const counts = q.base.map((c, i) => c + (picked.indexOf(i) >= 0 ? 1 : 0));
      // 多选按「选了该项的人数 / 总人数」算占比，单选按票数占比
      const denom = Math.max(1, q.type === 'single' ? counts.reduce((a, b) => a + b, 0) : total);
      const max = Math.max.apply(null, counts);
      const n = q.type === 'single' ? denom : total;
      return Object.assign(base, {
        n,
        bars: q.options.map((t, i) => ({ t, pct: (counts[i] / denom * 100).toFixed(1), mine: picked.indexOf(i) >= 0, top: counts[i] === max }))
      });
    }
    if (q.type === 'rate') {
      const sum = q.base.sum + (typeof my === 'number' ? my : 0);
      const n = q.base.n + (typeof my === 'number' ? 1 : 0);
      const avg = sum / n;
      // 演示：按均值附近的正态近似估算「高于你」的比例
      const myRate = typeof my === 'number' ? my : '-';
      const above = typeof my === 'number' ? Math.max(1, Math.min(99, Math.round(50 - (my - avg) * 18))) : '-';
      return Object.assign(base, { n, avg: avg.toFixed(1), myRate, above });
    }
    const texts = (q.samples || []).map(v => ({ v, mine: false }));
    if (my && String(my).trim()) texts.unshift({ v: my, mine: true });
    return Object.assign(base, { n: Math.round(total * 0.36), texts });
  },
  onHome() { wx.switchTab({ url: '/pages/index/index' }); },
  onShare() {
    wx.showActionSheet({ itemList: ['发送给微信好友', '生成分享海报', '复制问卷链接'], success: (r) => {
      wx.showToast({ title: ['已唤起分享', '海报已保存到相册', '链接已复制'][r.tapIndex], icon: 'none' });
    } });
  }
});
