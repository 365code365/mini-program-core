const store = require('../../utils/store.js');
const TAGS = ['家', '公司', '学校'];
const GENDERS = ['先生', '女士'];

Page({
  data: {
    id: '',
    name: '',
    gender: '先生',
    genders: GENDERS,
    phone: '',
    region: [],
    detail: '',
    house: '',
    tags: TAGS,
    tag: '',
    isDefault: false
  },
  onLoad(q) {
    const a = q.id ? store.getAddrs().find(x => x.id === q.id) : null;
    if (a) {
      this.setData({
        id: a.id, name: a.name, gender: a.gender || '', phone: a.phone, region: a.region || [], detail: a.detail,
        house: a.house || '', tag: a.tag || '', isDefault: !!a.isDefault
      });
      wx.setNavigationBarTitle({ title: '编辑收货地址' });
    } else {
      // 第一个地址自动设为默认
      this.setData({ isDefault: !store.getAddrs().length });
    }
  },
  onName(e) {
    this.setData({ name: e.detail.value });
  },
  onGender(e) {
    this.setData({ gender: e.currentTarget.dataset.g });
  },
  onPhone(e) {
    this.setData({ phone: e.detail.value });
  },
  onRegion(e) {
    this.setData({ region: e.detail.value });
  },
  onDetail(e) {
    this.setData({ detail: e.detail.value });
  },
  onHouse(e) {
    this.setData({ house: e.detail.value });
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
    const house = d.house.trim();
    let err = '';
    if (!name) err = '请填写联系人';
    else if (!/^1\d{10}$/.test(phone)) err = '请填写 11 位手机号';
    else if (d.region.length < 3) err = '请选择所在地区';
    else if (detail.length < 2) err = '请填写小区、写字楼或学校';
    else if (!house) err = '请填写门牌号';
    if (err) {
      wx.showToast({ title: err, icon: 'none' });
      return;
    }
    const a = store.upsertAddr({ id: d.id, name, gender: d.gender, phone, region: d.region, detail, house, tag: d.tag, isDefault: d.isDefault });
    // 新加的地址就是这一单要送的
    if (!d.id) store.pickAddr(a.id);
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
