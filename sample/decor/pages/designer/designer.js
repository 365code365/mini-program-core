const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { d: null, works: [], follow: false, starText: '' },

  onLoad(q) {
    const d = D.designer(q.id) || D.DESIGNERS[0];
    const works = D.CASES.filter((c) => c.designer === d.id);
    const reviews = d.reviews.map((r) => Object.assign({}, r, { stars: '★★★★★'.slice(0, r.star) + '☆☆☆☆☆'.slice(0, 5 - r.star) }));
    this.setData({ d: Object.assign({}, d, { reviews }), works, follow: store.get(store.K.favDesigners, []).indexOf(d.id) >= 0 });
    wx.setNavigationBarTitle({ title: d.name + ' · ' + d.title });
  },

  toggleFollow() {
    const on = store.toggle(store.K.favDesigners, this.data.d.id);
    this.setData({ follow: on });
    wx.showToast({ title: on ? '已关注，TA 有新作品会提醒你' : '已取消关注', icon: 'none' });
  },
  openCase(e) { wx.navigateTo({ url: '/pages/case/case?id=' + e.currentTarget.dataset.id }); },
  consult() {
    wx.showActionSheet({
      itemList: ['电话咨询 400-826-1688', '复制设计师微信号'],
      success: (r) => {
        if (r.tapIndex === 0) {
          wx.makePhoneCall({ phoneNumber: '4008261688', fail: () => wx.showToast({ title: '请拨打 400-826-1688', icon: 'none' }) });
        } else {
          wx.setClipboardData({ data: 'haozhai_' + this.data.d.id, success: () => wx.showToast({ title: '微信号已复制', icon: 'none' }) });
        }
      }
    });
  },
  book() { wx.navigateTo({ url: '/pages/booking/booking?designer=' + this.data.d.id }); }
});
