const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');
Page({
  data: { list: [], remindN: 0 },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    this.load();
  },
  load() {
    const now = Date.now();
    const reminds = store.reminds();
    const list = store.wants().map((id) => D.getShow(id)).filter(Boolean).map((s) => {
      const sale = D.onSale(s, now);
      const r = reminds[s.id];
      return {
        id: s.id, title: s.title, poster: s.poster, venue: s.city + ' · ' + s.venue, from: D.minPrice(s),
        date: s.sessions[0].date + (s.sessions.length > 1 ? ' 起' : ''), sale,
        saleText: sale ? '在售中' : F.monthDay(s.saleAt) + ' 开售',
        remind: r ? (r.how === 'wechat' ? '已订阅微信提醒' : '已设本机提醒') : ''
      };
    });
    this.setData({ list, remindN: Object.keys(reminds).length });
  },
  open(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消想看', content: '取消后也会关闭这场演出的开售提醒', confirmColor: '#ff3b5c',
      success: (r) => {
        if (!r.confirm) return;
        store.toggleWant(id); store.clearRemind(id); this.load();
      }
    });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
