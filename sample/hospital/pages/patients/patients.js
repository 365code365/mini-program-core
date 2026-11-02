const store = require('../../utils/store.js');
// 就诊人列表：点一位 → 编辑 / 设为默认 / 删除；新增与编辑在 patient-edit 页
Page({
  data: { list: [], max: store.MAX_PATIENTS },
  onShow() { this.refresh(); },
  refresh() {
    this.setData({ list: store.getPatients().map(p => Object.assign({}, p, { sx: p.name.charAt(0) })) });
  },
  goAdd() {
    if (this.data.list.length >= store.MAX_PATIENTS) {
      wx.showToast({ title: '最多添加 ' + store.MAX_PATIENTS + ' 位就诊人，可先删除不用的', icon: 'none' });
      return;
    }
    wx.navigateTo({ url: '/pages/patient-edit/patient-edit' });
  },
  onItem(e) {
    const p = store.findPatient(e.currentTarget.dataset.id);
    if (!p) return;
    wx.showActionSheet({
      itemList: ['编辑资料', p.isDefault ? '已是默认就诊人' : '设为默认就诊人', '删除就诊人'],
      success: r => {
        if (r.tapIndex === 0) {
          wx.navigateTo({ url: '/pages/patient-edit/patient-edit?id=' + p.id });
        } else if (r.tapIndex === 1) {
          if (!p.isDefault) store.setDefaultPatient(p.id);
          wx.showToast({ title: p.name + ' 已设为默认就诊人', icon: 'none' });
          this.refresh();
        } else {
          this.remove(p);
        }
      }
    });
  },
  remove(p) {
    wx.showModal({
      title: '删除就诊人',
      content: '确定删除「' + p.name + '」吗？已有的挂号与缴费记录仍会保留',
      confirmText: '删除',
      confirmColor: '#ff5a36',
      success: r => {
        if (!r.confirm) return;
        const msg = store.removePatient(p.id);
        if (msg) wx.showModal({ title: '暂时不能删除', content: msg, showCancel: false, confirmText: '知道了' });
        else wx.showToast({ title: '已删除', icon: 'none' });
        this.refresh();
      }
    });
  }
});
