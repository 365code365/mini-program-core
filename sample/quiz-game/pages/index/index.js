const G = require('../../utils/game.js');
const QS = require('../../utils/questions.js');

function mmss(ms) {
  const s = Math.ceil(ms / 1000);
  const m = Math.floor(s / 60);
  const r = s % 60;
  return (m < 10 ? '0' : '') + m + ':' + (r < 10 ? '0' : '') + r;
}
function today() { const d = new Date(); return d.getFullYear() + '-' + (d.getMonth() + 1) + '-' + d.getDate(); }

Page({
  data: { energy: 0, max: 10, cells: [], energyHint: '', tier: {}, tierHint: '', total: 0, props: {}, catList: [], qCount: 0, games: 0, best: 0, wrongCount: 0, signed: false },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const p = G.profile();
    const t = G.tier(p.total);
    const cats = G.cats();
    this.setData({
      total: p.total, tier: t, tierHint: t.next ? '距' + t.next + '还差 ' + t.need + ' 分' : '已是最高段位',
      props: p.props, games: p.games, best: p.best, wrongCount: G.wrongs().length,
      signed: G.get('signDay', '') === today(),
      catList: cats.map(k => { const c = QS.catOf(k); return { k, t: c.t, c: c.c, bg: QS.tint(c.c, 0.12) }; }),
      qCount: QS.byCats(cats).length
    });
    this.tick();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.tick(), 1000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },

  // 每秒算一次：倒计时走完时 energy() 自己把这一点加上
  tick() {
    const s = G.energy();
    const cells = [];
    for (let i = 0; i < s.max; i++) cells.push(i < s.energy ? 1 : 0);
    this.setData({ energy: s.energy, max: s.max, cells, energyHint: s.left ? mmss(s.left) + ' 后恢复 1 点' : '体力已满' });
  },

  start() {
    if (!G.useEnergy()) {
      return wx.showModal({ title: '体力不足', content: '体力每 5 分钟恢复 1 点，也可以点「补充体力」领取', confirmText: '补充体力', success: r => { if (r.confirm) this.refill(); } });
    }
    this.tick();
    wx.navigateTo({ url: '/pages/quiz/quiz' });
  },

  refill() {
    const n = G.get('refillDay', '') === today() ? G.get('refillCount', 0) : 0;
    if (n >= 3) return wx.showToast({ title: '今日补充次数已用完', icon: 'none' });
    if (this.data.energy >= this.data.max) return wx.showToast({ title: '体力已满', icon: 'none' });
    wx.showModal({
      title: '补充体力',
      content: '观看 15 秒广告可获得 2 点体力（今日剩余 ' + (3 - n) + ' 次）',
      confirmText: '看广告',
      success: r => {
        if (!r.confirm) return;
        wx.showLoading({ title: '广告播放中' });
        setTimeout(() => {
          wx.hideLoading();
          G.addEnergy(2);
          G.set('refillDay', today());
          G.set('refillCount', n + 1);
          this.tick();
          wx.showToast({ title: '体力 +2', icon: 'success' });
        }, 800);
      }
    });
  },

  sign() {
    if (this.data.signed) return wx.showToast({ title: '今天已经签过啦', icon: 'none' });
    const p = G.profile();
    p.props.half += 1;
    p.props.skip += 1;
    G.saveProfile(p);
    G.set('signDay', today());
    this.setData({ signed: true, props: p.props });
    wx.showToast({ title: '50:50 与跳过各 +1', icon: 'none' });
  },

  pickCats() { wx.navigateTo({ url: '/pages/category/category' }); }
});
