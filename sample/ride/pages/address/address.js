// 常用地址：家 / 公司各一个、其他最多 5 个（store.getPlaces）。设置 / 修改走选地点页（place?type=home|company|other），
// 点地址本身 = 把它设成目的地回首页叫车
const store = require('../../utils/store.js');
const SLOT = { home: '家', company: '公司' };

Page({
  data: { home: null, company: null, others: [], max: 5 },
  onShow() {
    const v = store.getPlaces();
    this.setData({ home: v.home || null, company: v.company || null, others: v.others || [] });
  },
  onSet(e) {
    const slot = e.currentTarget.dataset.slot;
    if (slot === 'other' && this.data.others.length >= this.data.max) {
      wx.showToast({ title: '其他常用地址最多 5 个，先删掉一个吧', icon: 'none' });
      return;
    }
    wx.navigateTo({ url: '/pages/place/place?type=' + slot });
  },
  pickOf(e) {
    const slot = e.currentTarget.dataset.slot;
    const i = Number(e.currentTarget.dataset.i);
    return { slot, i, p: slot === 'other' ? this.data.others[i] : this.data[slot] };
  },
  onRemove(e) {
    const { slot, i, p } = this.pickOf(e);
    if (!p) return;
    wx.showModal({
      title: '删除常用地址',
      content: '确定删除「' + (SLOT[slot] ? SLOT[slot] + '：' : '') + p.name + '」吗？',
      confirmText: '删除',
      confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        store.removePlace(slot, i);
        this.onShow();
        wx.showToast({ title: '已删除', icon: 'none' });
      }
    });
  },
  onGo(e) {
    const { p } = this.pickOf(e);
    if (!p) return;
    if (store.getFrom().name === p.name) {
      wx.showToast({ title: '你就在这里，换一个目的地吧', icon: 'none' });
      return;
    }
    store.setTo(p);
    wx.switchTab({ url: '/pages/index/index' });
  }
});
