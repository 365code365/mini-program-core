const SCORES = [96, 72, 45, 88, 60];

Page({
  data: {
    score: SCORES[0],
    goods: [
      { id: 1, name: '手冲咖啡', fen: 2800, tags: ['热饮', '新品'] },
      { id: 2, name: '降噪耳机', fen: 129900, tags: ['数码'] },
      { id: 3, name: '草莓蛋糕', fen: 6800, tags: ['甜品', '限量', '冷藏'] }
    ],
    spec: { 颜色: '曜石黑', 容量: '256GB', 产地: '深圳' },
    keyed: [
      { id: 'a', name: '项目 A', on: false },
      { id: 'b', name: '项目 B', on: true },
      { id: 'c', name: '项目 C', on: false },
      { id: 'd', name: '项目 D', on: false }
    ],
    user: { name: '小程序开发者', city: '深圳', phone: '13912345678', avatar: '/images/avatar.jpg', vip: true },
    tpl: 'tplA',
    now: Date.now(),
    a: 3,
    b: 4,
    highlight: false
  },
  scoreAt: 0,
  nextKey: 5,
  nextScore() {
    this.scoreAt = (this.scoreAt + 1) % SCORES.length;
    this.setData({ score: SCORES[this.scoreAt] });
  },
  shuffle() {
    const list = this.data.keyed.slice();
    for (let i = list.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      const t = list[i]; list[i] = list[j]; list[j] = t;
    }
    this.setData({ keyed: list });
  },
  addItem() {
    const id = 'n' + this.nextKey++;
    this.setData({ keyed: [{ id, name: '新项 ' + id, on: true }].concat(this.data.keyed) });
  },
  switchTpl() {
    this.setData({ tpl: this.data.tpl === 'tplA' ? 'tplB' : 'tplA' });
  },
  toggleHl() {
    this.setData({ highlight: !this.data.highlight });
  },
  go(e) {
    wx.navigateTo({ url: e.currentTarget.dataset.url });
  }
});
