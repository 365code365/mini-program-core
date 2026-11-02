const db = require('../../utils/db.js');

Page({
  data: { list: [], select: false, cur: '' },
  onLoad(q) {
    this.setData({ select: q.select === '1', cur: q.cur || '' });
    if (q.select === '1') wx.setNavigationBarTitle({ title: '选择服务地址' });
  },
  onShow() { this.load(); },
  load() { this.setData({ list: db.getAddrs().map((a) => Object.assign({}, a, { full: db.fullAddr(a) })) }); },
  onTap(e) {
    const id = e.currentTarget.dataset.id;
    if (!this.data.select) { this.edit(e); return; }
    db.setPicked(id);
    wx.navigateBack();
  },
  edit(e) { wx.navigateTo({ url: '/pages/address-edit/address-edit?id=' + e.currentTarget.dataset.id }); },
  setDefault(e) { db.setDefaultAddr(e.currentTarget.dataset.id); this.load(); wx.showToast({ title: '已设为默认地址', icon: 'none' }); },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除地址', content: '删除后不可恢复，确定删除？', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { db.removeAddr(id); this.load(); } }
    });
  },
  // 选择模式下新增：保存后直接回到下单页（address-edit 里 navigateBack delta 2）
  add() { wx.navigateTo({ url: '/pages/address-edit/address-edit' + (this.data.select ? '?from=select' : '') }); }
});
