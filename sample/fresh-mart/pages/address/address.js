const store = require('../../utils/store.js');
const host = require('../../utils/host.js');

Page({
  data: {
    select: false,
    pickedId: '',
    list: []
  },

  onLoad(q) {
    const select = q.select === '1';
    this.setData({ select });
    if (select) wx.setNavigationBarTitle({ title: '选择收货地址' });
  },

  onShow() {
    const cur = store.currentAddr();
    this.setData({
      pickedId: cur ? cur.id : '',
      list: store.getAddrs().map(store.addrView)
    });
  },

  onPick(e) {
    if (!this.data.select) {
      this.onEdit(e);
      return;
    }
    const a = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    if (!a.inRange) {
      wx.showToast({ title: '超出配送范围，前置仓只送北京市内', icon: 'none' });
      return;
    }
    store.pickAddr(a.id);
    wx.navigateBack();
  },

  onEdit(e) {
    wx.navigateTo({ url: '/pages/address-edit/address-edit?id=' + e.currentTarget.dataset.id });
  },

  onAdd() {
    wx.navigateTo({ url: '/pages/address-edit/address-edit' });
  },

  onImport() {
    host.chooseAddress().then(r => {
      if (!r) return;
      const a = store.upsertAddr(Object.assign(r, { tag: '', isDefault: false }));
      wx.showToast({ title: '已导入微信地址', icon: 'none' });
      if (this.data.select && store.inRange(a)) {
        store.pickAddr(a.id);
        wx.navigateBack();
        return;
      }
      this.onShow();
    });
  }
});
