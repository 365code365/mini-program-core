const store = require('../../utils/store.js');
const TAGS = ['家', '公司', '学校'];

Page({
  data: {
    id: '',
    name: '',
    phone: '',
    region: [],
    detail: '',
    tags: TAGS,
    tag: '',
    isDefault: false
  },

  onLoad(q) {
    const a = q.id ? store.getAddrs().find(x => x.id === q.id) : null;
    if (a) {
      this.setData({ id: a.id, name: a.name, phone: a.phone, region: a.region || [], detail: a.detail, tag: a.tag || '', isDefault: !!a.isDefault });
      wx.setNavigationBarTitle({ title: '编辑收货地址' });
    } else {
      // 第一个地址自动设为默认
      this.setData({ isDefault: !store.getAddrs().length });
    }
  },

  onName(e) {
    this.setData({ name: e.detail.value });
  },

  onPhone(e) {
    this.setData({ phone: e.detail.value });
  },

  onRegion(e) {
    this.setData({ region: e.detail.value });
    if (e.detail.value[1] !== '北京市') wx.showToast({ title: '前置仓暂时只送北京市内，外地地址可保存但不能下单', icon: 'none' });
  },

  onDetail(e) {
    this.setData({ detail: e.detail.value });
  },

  onTag(e) {
    const t = e.currentTarget.dataset.tag;
    this.setData({ tag: this.data.tag === t ? '' : t });
  },

  onDefault(e) {
    this.setData({ isDefault: e.detail.value });
  },

  onSave() {
    const d = this.data;
    const name = d.name.trim();
    const phone = d.phone.trim();
    const detail = d.detail.trim();
    let err = '';
    if (!name) err = '请填写收货人';
    else if (!/^1\d{10}$/.test(phone)) err = '请填写 11 位手机号';
    else if (d.region.length < 3) err = '请选择所在地区';
    else if (detail.length < 4) err = '详细地址至少 4 个字';
    if (err) {
      wx.showToast({ title: err, icon: 'none' });
      return;
    }
    store.upsertAddr({ id: d.id, name, phone, region: d.region, detail, tag: d.tag, isDefault: d.isDefault });
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 400);
  },

  onDelete() {
    wx.showModal({
      title: '删除地址',
      content: '确定删除这个收货地址吗？',
      confirmText: '删除',
      confirmColor: '#ff4d2d',
      success: r => {
        if (!r.confirm) return;
        store.removeAddr(this.data.id);
        wx.navigateBack();
      }
    });
  }
});
