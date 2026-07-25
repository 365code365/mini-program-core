// 能力展示：把画廊里的组件与动画能力在商城里跑一遍（全部可交互）
Page({
  data: {
    notices: ['📢 今日下单最快次日达', '🎁 会员日全场 8 折', '🚚 满 99 元包邮', '⭐️ 好评返 5 元'],
    pushOn: true,
    volume: 60,
    slots: ['今天 18:00-20:00', '明天 09:00-12:00', '明天 14:00-18:00', '不限时段'],
    slotIndex: 0,
    tastes: ['微辣', '不要香菜', '少糖'],
    invoices: ['个人', '企业', '不需要'],
    invoice: '个人',
    note: '',
    progress: 40,
    rating: 4,
    badge: 3,
    sheetVisible: false,
    sheetActions: ['分享给好友', '收藏商品', '举报'],
    richNodes: []
  },

  onLoad: function () {
    this.setData({
      richNodes: [
        { name: 'p', attrs: { style: 'font-size:13px;color:#666;line-height:1.9' },
          children: [{ type: 'text', text: 'rich-text 支持按节点树渲染带样式的文本片段：' }] },
        { name: 'p', attrs: { style: 'font-size:15px;color:#FF6B35;line-height:1.9' },
          children: [{ type: 'text', text: '加粗大号橙色标题' }] },
        { name: 'p', attrs: { style: 'font-size:12px;color:#999;line-height:1.9' },
          children: [{ type: 'text', text: '小号灰色说明文字 · 与原生文本同一套排版' }] }
      ]
    });
  },

  onReady: function () {
    this.drawChart();
  },

  // Canvas 2D 柱状图
  drawChart: function () {
    var ctx = wx.createCanvasContext('showcaseChart');
    var data = [42, 68, 55, 90, 74, 61, 83];
    var labels = ['一', '二', '三', '四', '五', '六', '日'];
    var w = 300, h = 140, base = 118, bw = 24, gap = 16;
    ctx.setFillStyle('#FAFAFA');
    ctx.fillRect(0, 0, w, h);
    // 网格线
    ctx.setStrokeStyle('#EEEEEE');
    ctx.setLineWidth(1);
    for (var g = 0; g <= 3; g++) {
      var gy = 20 + g * 32;
      ctx.beginPath();
      ctx.moveTo(10, gy);
      ctx.lineTo(w - 10, gy);
      ctx.stroke();
    }
    // 柱子
    for (var i = 0; i < data.length; i++) {
      var bh = data[i] * 0.9;
      var x = 20 + i * (bw + gap);
      ctx.setFillStyle(i === 3 ? '#FF3B30' : '#FF6B35');
      ctx.fillRect(x, base - bh, bw, bh);
      ctx.setFillStyle('#999999');
      ctx.setFontSize(10);
      ctx.fillText(labels[i], x + 6, base + 14);
    }
    ctx.draw();
  },

  // ==================== 表单交互 ====================

  onPushChange: function (e) {
    this.setData({ pushOn: e.detail.value });
    wx.showToast({ title: e.detail.value ? '已开启推送' : '已关闭推送', icon: 'none' });
  },

  onVolume: function (e) {
    this.setData({ volume: e.detail.value });
  },

  onSlot: function (e) {
    this.setData({ slotIndex: parseInt(e.detail.value) || 0 });
  },

  onTaste: function (e) {
    var picked = e.detail.value || [];
    wx.showToast({ title: picked.length ? picked.join('/') : '已清空口味', icon: 'none' });
  },

  onInvoice: function (e) {
    this.setData({ invoice: e.detail.value });
  },

  onNote: function (e) {
    this.setData({ note: e.detail.value });
  },

  onProgressStep: function (e) {
    var d = parseInt(e.currentTarget.dataset.d);
    var next = Math.max(0, Math.min(100, this.data.progress + d));
    this.setData({ progress: next });
  },

  onRate: function (e) {
    this.setData({ rating: parseInt(e.currentTarget.dataset.n) });
  },

  onBadge: function () {
    this.setData({ badge: this.data.badge + 1 });
  },

  // ==================== 弹层 ====================

  onToast: function () {
    wx.showToast({ title: '操作成功', icon: 'success' });
  },

  onLoading: function () {
    wx.showLoading({ title: '处理中...' });
    setTimeout(function () {
      wx.hideLoading();
      wx.showToast({ title: '完成', icon: 'success' });
    }, 900);
  },

  onModal: function () {
    wx.showModal({
      title: '确认操作',
      content: '这是一个带取消/确定的模态对话框，两端行为一致。',
      success: function (res) {
        wx.showToast({ title: res.confirm ? '你点了确定' : '你点了取消', icon: 'none' });
      }
    });
  },

  onSheet: function () { this.setData({ sheetVisible: true }); },
  onCloseSheet: function () { this.setData({ sheetVisible: false }); },
  onSheetAction: function (e) {
    this.setData({ sheetVisible: false });
    wx.showToast({ title: e.currentTarget.dataset.k, icon: 'none' });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
