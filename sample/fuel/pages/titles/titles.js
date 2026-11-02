const store = require('../../utils/store.js');

Page({
  data: { list: [], select: false },
  onLoad(q) { this.setData({ select: q.select === '1' }); },
  onShow() { this.setData({ list: store.titles() }); },
  onPick(e) {
    if (!this.data.select) { this.edit(e); return; }
    const t = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    this.getOpenerEventChannel().emit('picked', t);
    wx.navigateBack();
  },
  setDefault(e) {
    const t = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    if (t.isDefault) return;
    this.setData({ list: store.saveTitle(Object.assign({}, t, { isDefault: true })) });
    wx.showToast({ title: '已设为默认', icon: 'success' });
  },
  edit(e) { wx.navigateTo({ url: '/pages/title-edit/title-edit?id=' + e.currentTarget.dataset.id }); },
  add() { wx.navigateTo({ url: '/pages/title-edit/title-edit' }); },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({ title: '删除抬头', content: '删除后不影响已开具的发票', success: (r) => { if (r.confirm) this.setData({ list: store.removeTitle(id) }); } });
  },
  fromWechat() {
    wx.chooseInvoiceTitle({
      success: (res) => {
        this.setData({ list: store.saveTitle({ type: res.type === 0 || res.type === '0' ? 'company' : 'person', name: res.title, taxNo: res.taxNumber || '', email: '', isDefault: false }) });
        wx.showToast({ title: '已导入', icon: 'success' });
      },
      fail: () => wx.showModal({ title: '无法读取微信发票抬头', content: '当前环境不支持，请手动新增抬头', confirmText: '去新增', success: (r) => r.confirm && this.add() })
    });
  }
});
