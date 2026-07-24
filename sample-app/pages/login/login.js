Page({
  data: {
    phone: '',
    code: '',
    agreed: false,
    counting: false,
    countdown: 60
  },

  onToggleAgree: function() {
    this.setData({ agreed: !this.data.agreed });
  },

  onSendCode: function() {
    if (this.data.counting) return;
    if (!/^1\d{10}$/.test(this.data.phone)) {
      wx.showToast({ title: '请输入正确手机号', icon: 'none' });
      return;
    }
    var self = this;
    this.setData({ counting: true, countdown: 60 });
    wx.showToast({ title: '验证码已发送', icon: 'success' });
    var timer = setInterval(function() {
      var c = self.data.countdown - 1;
      if (c <= 0) {
        clearInterval(timer);
        self.setData({ counting: false, countdown: 60 });
      } else {
        self.setData({ countdown: c });
      }
    }, 1000);
  },

  onLogin: function() {
    if (!/^1\d{10}$/.test(this.data.phone)) {
      wx.showToast({ title: '请输入正确手机号', icon: 'none' });
      return;
    }
    if (!this.data.code) {
      wx.showToast({ title: '请输入验证码', icon: 'none' });
      return;
    }
    if (!this.data.agreed) {
      wx.showToast({ title: '请先同意用户协议', icon: 'none' });
      return;
    }
    wx.setStorageSync('user', { phone: this.data.phone, nickname: '用户' + this.data.phone.slice(-4) });
    wx.showToast({ title: '登录成功', icon: 'success' });
    setTimeout(function() { wx.navigateBack(); }, 800);
  },

  onWechatLogin: function() {
    wx.setStorageSync('user', { phone: '', nickname: '微信用户' });
    wx.showToast({ title: '登录成功', icon: 'success' });
    setTimeout(function() { wx.navigateBack(); }, 800);
  }
});
