const G = require('../../utils/game.js');

const COLORS = ['#F59E0B', '#10B981', '#3B82F6', '#EC4899', '#EF4444', '#8B5CF6', '#14B8A6', '#F97316'];
const HEAD_RPX = 420;
const TABBAR_RPX = 100;
const ROW_RPX = 112;

Page({
  data: { kind: 'today', podium: [], rest: [], me: {}, pin: '', into: '', listH: 400 },

  onLoad() {
    const sys = wx.getSystemInfoSync();
    this.k = sys.windowWidth / 750;
    this.rowH = ROW_RPX * this.k;
    // scroll-view 不做 position:fixed：固定定位的 scroll-view 滚动后点不中里面的行（引擎问题），所以按窗口高度算死高度
    this.setData({ listH: Math.floor(sys.windowHeight - (HEAD_RPX + TABBAR_RPX) * this.k - (sys.safeArea ? sys.windowHeight - sys.safeArea.bottom : 0)) });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.load(this.data.kind);
  },

  load(kind) {
    const b = G.board(kind);
    const list = b.list.map((x, i) => Object.assign({}, x, { first: x.me ? '我' : x.name.slice(0, 1), color: x.me ? '#5B4BF5' : COLORS[i % COLORS.length] }));
    const top = list.slice(0, 3);
    // 领奖台排成 2 1 3
    const podium = [top[1], top[0], top[2]].map((x, i) => Object.assign({}, x, { pos: [2, 1, 3][i], h: [110, 150, 80][i], name: x.me ? '我' : x.name }));
    const rest = list.slice(3);
    const me = list.find(x => x.me);
    this.meIdx = rest.findIndex(x => x.me);
    this.scrollTop = 0;
    this.setData({ kind, podium, rest, me, into: '' });
    this.updatePin(0);
  },

  onTab(e) {
    const k = e.currentTarget.dataset.k;
    if (k !== this.data.kind) this.load(k);
  },

  onScroll(e) {
    this.scrollTop = e.detail.scrollTop;
    this.updatePin(this.scrollTop);
  },

  updatePin(top) {
    let pin = '';
    if (this.meIdx >= 0) {
      const y = this.meIdx * this.rowH;
      if (y < top) pin = 'top';
      else if (y + this.rowH > top + this.data.listH) pin = 'bottom';
    }
    if (pin !== this.data.pin) this.setData({ pin });
  },

  locate() {
    // 先清空再设，同一个 id 连点也能再次定位
    this.setData({ into: '' });
    this.setData({ into: 'me' });
  }
});
