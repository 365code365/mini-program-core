const store = require('../../utils/store.js');

function mask(p) { return p.slice(0, 3) + '****' + p.slice(7); }
const EMPTY = { id: '', name: '', phone: '', region: '', regionArr: [], community: '', room: '' };

// ?select=1：从下单页进来，点一条就选中并返回
Page({
  data: { list: [], selId: '', select: false, editing: false, form: EMPTY, err: '' },

  onLoad(q) {
    this.setData({ select: q.select === '1' });
    this.render();
  },

  render() {
    const list = store.addrs().map(a => Object.assign({}, a, { phoneMask: mask(a.phone) }));
    const sel = store.selectedAddr();
    this.setData({ list, selId: sel ? sel.id : '' });
  },

  onPick(e) {
    store.selectAddr(e.currentTarget.dataset.id);
    this.render();
    if (this.data.select) wx.navigateBack();
  },

  add() { this.setData({ editing: true, form: Object.assign({}, EMPTY), err: '' }); },

  edit(e) {
    const a = store.addrs().find(x => x.id === e.currentTarget.dataset.id);
    if (a) this.setData({ editing: true, form: Object.assign({}, EMPTY, a), err: '' });
  },

  cancelEdit() { this.setData({ editing: false }); },

  // 微信收货地址：桌面端没有宿主时走 fail，提示手动填写
  fromWx() {
    wx.chooseAddress({
      success: r => this.setData({
        editing: true, err: '',
        form: Object.assign({}, EMPTY, {
          name: r.userName, phone: r.telNumber, regionArr: [r.provinceName, r.cityName, r.countyName],
          region: [r.provinceName, r.cityName, r.countyName].join(' '), community: r.detailInfo, room: ''
        })
      }),
      fail: () => {
        wx.showToast({ title: '无法读取微信地址，请手动填写', icon: 'none' });
        this.add();
      }
    });
  },

  onField(e) { this.setData({ ['form.' + e.currentTarget.dataset.k]: e.detail.value, err: '' }); },

  onRegion(e) {
    const v = e.detail.value;
    this.setData({ 'form.regionArr': v, 'form.region': v.join(' '), err: '' });
  },

  save() {
    const f = this.data.form;
    if (!f.name.trim()) return this.setData({ err: '请填写联系人' });
    if (!/^1[3-9]\d{9}$/.test(f.phone)) return this.setData({ err: '手机号格式不正确' });
    if (!f.region) return this.setData({ err: '请选择所在地区' });
    if (f.community.trim().length < 2) return this.setData({ err: '请填写小区 / 楼栋' });
    if (!f.room.trim()) return this.setData({ err: '请填写门牌号，方便取件员找到你' });
    const a = { id: f.id || 'a' + Date.now(), name: f.name.trim(), phone: f.phone, region: f.region, regionArr: f.regionArr, community: f.community.trim(), room: f.room.trim() };
    store.saveAddr(a);
    if (!this.data.selId || this.data.select) store.selectAddr(a.id);
    this.setData({ editing: false });
    this.render();
    wx.showToast({ title: '地址已保存', icon: 'success' });
    if (this.data.select) setTimeout(() => wx.navigateBack(), 400);
  },

  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除这个地址？',
      confirmColor: '#ff4d3a',
      success: res => {
        if (!res.confirm) return;
        store.removeAddr(id);
        this.render();
      }
    });
  }
});
