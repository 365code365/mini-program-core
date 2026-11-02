const data = require('../../utils/spots.js');
const store = require('../../utils/store.js');

Page({
  data: { spot: {}, markers: [], circles: [], tips: [], fav: false, inRoute: false, guideNote: '' },
  onLoad(q) {
    const s = data.byId(q.id) || data.SPOTS[0];
    wx.setNavigationBarTitle({ title: s.name });
    this.setData({
      spot: s,
      tips: s.tips,
      markers: data.markers([s], s.id),
      circles: [{ latitude: s.lat, longitude: s.lng, radius: 300, color: '#1c3d5a66', fillColor: '#1c3d5a14', strokeWidth: 1 }]
    });
  },
  onShow() {
    const id = this.data.spot.id;
    this.setData({ fav: store.get('fav', []).indexOf(id) >= 0, inRoute: store.get('route', []).indexOf(id) >= 0 });
    // 攻略页（web-view）里点了「收藏这篇」：网页 postMessage，返回时由 bindmessage 带回来
    const msgs = getApp().globalData.guideMessages.filter((m) => m && m.spotId === id);
    if (msgs.length) {
      const last = msgs[msgs.length - 1];
      if (last.action === 'fav' && !this.data.fav) {
        store.toggleFav(id);
        this.setData({ fav: true });
      }
      this.setData({ guideNote: '在攻略里' + (last.action === 'fav' ? '收藏了本地点' : '读完了「' + last.title + '」') + '（共 ' + msgs.length + ' 条消息）' });
    }
  },
  toggleFav() {
    const on = store.toggleFav(this.data.spot.id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  addRoute() {
    if (store.addToRoute(this.data.spot.id)) {
      this.setData({ inRoute: true });
      wx.showToast({ title: '已加入路线', icon: 'success' });
    } else {
      wx.switchTab({ url: '/pages/route/route' });
    }
  },
  openGuide() {
    wx.navigateTo({ url: '/pages/guide/guide?id=' + this.data.spot.id });
  },
  openLive() {
    wx.navigateTo({ url: '/pages/live/live?id=' + this.data.spot.id });
  },
  openLocation(e) {
    // 宿主接了地图时点击带经纬度；没接（桌面预览）时引擎按普通节点派、只有 x / y：两种都打开导航
    const s = this.data.spot;
    wx.openLocation({
      latitude: s.lat, longitude: s.lng, name: s.name, address: s.address, scale: 16,
      fail: () => wx.showModal({ title: s.name, content: s.address + '\n（当前环境打不开系统地图）', showCancel: false })
    });
  }
});
