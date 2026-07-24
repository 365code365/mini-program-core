Page({
  data: {
    addresses: [
      { id: 1, name: '张三', phone: '13800001111', region: '广东省深圳市南山区', detail: '科技园路 1 号', isDefault: true }
    ],
    selectedId: 1,
    name: '',
    phone: '',
    region: '',
    detail: '',
    setDefault: false,
    nextId: 2
  },

  onSelect: function(e) {
    this.setData({ selectedId: e.currentTarget.dataset.id });
  },

  onDelete: function(e) {
    var id = e.currentTarget.dataset.id;
    var list = this.data.addresses.filter(function(a) { return a.id !== id; });
    this.setData({ addresses: list });
    wx.showToast({ title: '已删除', icon: 'none' });
  },

  onDefaultChange: function(e) {
    this.setData({ setDefault: e.detail.value });
  },

  onSave: function() {
    if (!this.data.name) { wx.showToast({ title: '请填写收货人', icon: 'none' }); return; }
    if (!/^1\d{10}$/.test(this.data.phone)) { wx.showToast({ title: '手机号有误', icon: 'none' }); return; }
    if (!this.data.detail) { wx.showToast({ title: '请填写详细地址', icon: 'none' }); return; }

    var id = this.data.nextId;
    var list = this.data.addresses.slice();
    if (this.data.setDefault) {
      list = list.map(function(a) { a.isDefault = false; return a; });
    }
    list.push({
      id: id,
      name: this.data.name,
      phone: this.data.phone,
      region: this.data.region,
      detail: this.data.detail,
      isDefault: this.data.setDefault
    });
    this.setData({
      addresses: list,
      nextId: id + 1,
      name: '', phone: '', region: '', detail: '', setDefault: false
    });
    wx.showToast({ title: '保存成功', icon: 'success' });
  }
});
