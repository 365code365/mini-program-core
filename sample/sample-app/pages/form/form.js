Page({
  data: {
    nickname: '小明',
    bio: '',
    search: '',
    count: 0,
    pushOn: true,
    volume: 60,
    hobbies: [
      { label: '阅读', value: 'read', checked: true },
      { label: '运动', value: 'sport', checked: false },
      { label: '音乐', value: 'music', checked: true }
    ],
    hobbyText: '阅读、音乐',
    gender: 'male',
    submitted: ''
  },

  onLoad: function () {
    console.log('form page loaded');
  },

  // bindinput：输入即回调
  onSearchInput: function (e) {
    this.setData({ search: e.detail.value });
  },

  // 计数器
  onInc: function () { this.setData({ count: this.data.count + 1 }); },
  onDec: function () { this.setData({ count: Math.max(0, this.data.count - 1) }); },
  onReset: function () { this.setData({ count: 0 }); },

  // switch / slider
  onPushChange: function (e) { this.setData({ pushOn: e.detail.value }); },
  onVolume: function (e) { this.setData({ volume: e.detail.value }); },

  // 多选：收集选中项
  onHobby: function (e) {
    var vals = e.detail.value; // 选中值数组
    var labels = this.data.hobbies
      .filter(function (h) { return vals.indexOf(h.value) >= 0; })
      .map(function (h) { return h.label; });
    this.setData({ hobbyText: labels.join('、') || '无' });
  },

  // 单选
  onGender: function (e) { this.setData({ gender: e.detail.value }); },

  // 表单提交
  onSubmit: function (e) {
    var v = e.detail.value || {};
    this.setData({ submitted: JSON.stringify(v) });
    wx.showToast({ title: '提交成功', icon: 'success' });
  },
  onReset2: function () {
    this.setData({ submitted: '' });
  },

  onBack: function () {
    wx.switchTab({ url: '/pages/index/index' });
  },
  onNavHome: function () {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
