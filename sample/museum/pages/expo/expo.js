// 特展详情：横幅、展期与状态、rich-text 介绍、展品清单；进行中的特展可直接去预约，未开展的可设开展提醒
const { expos, ymd } = require('../../utils/data.js');
const U = require('../../utils/util.js');

Page({
  data: { x: null, status: '', items: [], reminded: false, daysLeft: 0 },
  onLoad(q) {
    const x = expos().find(e => e.id === q.id) || expos()[0];
    const today = ymd(new Date());
    const ms = 86400000;
    const status = x.start > today ? '即将开展' : '展出中';
    const daysLeft = status === '展出中'
      ? Math.round((new Date(x.end.replace(/-/g, '/')) - new Date(today.replace(/-/g, '/'))) / ms)
      : Math.round((new Date(x.start.replace(/-/g, '/')) - new Date(today.replace(/-/g, '/'))) / ms);
    this.setData({ x, status, daysLeft, items: x.exhibits.map(id => U.exhibit(id)) });
  },
  onShow() { this.setData({ reminded: U.load('mu_expo_remind', []).indexOf(this.data.x.id) >= 0 }); },
  openExhibit(e) { wx.navigateTo({ url: '/pages/exhibit/exhibit?id=' + e.currentTarget.dataset.id }); },
  book() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  remind() {
    const list = U.load('mu_expo_remind', []);
    const id = this.data.x.id;
    const on = list.indexOf(id) < 0;
    U.save('mu_expo_remind', on ? list.concat(id) : list.filter(k => k !== id));
    this.setData({ reminded: on });
    // 订阅消息需要宿主；拿不到就只在本地记一笔，开展当天首页提示
    wx.requestSubscribeMessage({
      tmplIds: ['expo_open_notice'],
      complete: () => wx.showToast({ title: on ? '已设置开展提醒' : '已取消提醒', icon: 'none' })
    });
  },
  onShareAppMessage() { return { title: this.data.x.title + ' · ' + this.data.x.sub, path: '/pages/expo/expo?id=' + this.data.x.id }; }
});
