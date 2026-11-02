const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { projects: [], feed: [], tab: 'mine' },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    this.load();
  },

  load() {
    const list = store.projects();
    const projects = list.map((p) => ({
      id: p.id, community: p.community, area: p.area, houseType: p.houseType, no: p.no,
      stage: p.finished ? '已完工' : store.STAGES[p.stage], percent: store.percent(p), waiting: p.stage === 0,
      count: (p.diaries || []).length
    }));
    const feed = [];
    list.forEach((p) => (p.diaries || []).forEach((y) => feed.push({
      id: y.id, pid: p.id, ts: y.ts, time: store.ymdhm(y.ts), text: y.text, photo: y.photo,
      stageName: store.STAGES[y.stage], where: p.community.split(' ').slice(0, 2).join(' ')
    })));
    feed.sort((a, b) => b.ts - a.ts);
    this.setData({ projects, feed });
  },

  setTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  open(e) { wx.navigateTo({ url: '/pages/progress/progress?id=' + e.currentTarget.dataset.id }); },
  book() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  goCases() { wx.switchTab({ url: '/pages/index/index' }); },
  preview(e) { wx.previewImage({ urls: [e.currentTarget.dataset.src] }); }
});
