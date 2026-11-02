const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 收货地址：?select=1 从购物车进来是「选一个」，否则是管理
Page({
  data: { select: false, pickedId: '', list: [] },
  onLoad(q) {
    const select = q.select === '1';
    this.setData({ select });
    if (select) wx.setNavigationBarTitle({ title: '选择收货地址' });
  },
  onShow() {
    const cur = store.currentAddr();
    this.setData({
      pickedId: cur ? cur.id : '',
      list: store.getAddrs().map(a => Object.assign({}, a, { phoneMask: store.maskPhone(a.phone), regionText: a.region.join(' ') }))
    });
  },
  onPick(e) {
    if (!this.data.select) return this.onEdit(e);
    store.pickAddr(e.currentTarget.dataset.id);
    wx.navigateBack();
  },
  onEdit(e) { wx.navigateTo({ url: '/pages/address-edit/address-edit?id=' + e.currentTarget.dataset.id }); },
  onAdd() { wx.navigateTo({ url: '/pages/address-edit/address-edit' }); },
  onImport() {
    host.chooseAddress().then(r => {
      if (!r) return;
      const a = store.upsertAddr(Object.assign(r, { tag: '', isDefault: false }));
      wx.showToast({ title: '已导入微信地址', icon: 'none' });
      if (this.data.select) {
        store.pickAddr(a.id);
        wx.navigateBack();
        return;
      }
      this.onShow();
    });
  }
});
