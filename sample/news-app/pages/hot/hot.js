// 头条新闻 · 热榜 / 签到日历 / 阅读统计（canvas 图表）
var RANK = {
  day: [
    { id: 1001, title: '数字经济规模再创新高，实体产业融合成为主线', heat: 210 },
    { id: 301, title: '联赛焦点战：主队补时绝杀 现场万人沸腾', heat: 188 },
    { id: 201, title: '新能源车渗透率突破关键节点', heat: 165 },
    { id: 501, title: '实拍：国产大飞机完成高原试飞全过程', heat: 143 },
    { id: 101, title: '国产大飞机再获百架订单', heat: 128 },
    { id: 102, title: '多地推出住房消费新政', heat: 96 },
    { id: 1002, title: '城市更新样本：老街区如何焕发新活力', heat: 88 }
  ],
  week: [
    { id: 301, title: '联赛焦点战：主队补时绝杀 现场万人沸腾', heat: 642 },
    { id: 1001, title: '数字经济规模再创新高，实体产业融合成为主线', heat: 588 },
    { id: 401, title: '暑期档票房破百亿 现实题材成最大赢家', heat: 473 },
    { id: 201, title: '新能源车渗透率突破关键节点', heat: 412 },
    { id: 601, title: '新型运输机完成高原起降科目训练', heat: 366 },
    { id: 701, title: '多国联合发布气候行动倡议', heat: 291 }
  ]
};

Page({
  data: {
    weekdays: ['一', '二', '三', '四', '五', '六', '日'],
    calendar: [],
    streak: 0,
    nextReward: 5,
    checkedToday: false,
    range: 'day',
    ranking: RANK.day,
    readMinutes: [22, 35, 18, 46, 31, 52, 28],
    weekTotal: 0,
    weekAvg: 0,
    goalPercent: 0,
    favorites: []
  },

  onLoad: function () {
    this.buildCalendar();
    this.computeStats();
    this.setData({ favorites: getApp().getFavorites() });
    // 榜单条形宽度按热度归一
    this.applyRange('day');
  },

  applyRange: function (k) {
    var list = (RANK[k] || RANK.day).slice();
    var max = list[0] ? list[0].heat : 1;
    this.setData({
      range: k,
      ranking: list.map(function (r) {
        return { id: r.id, title: r.title, heat: r.heat, percent: Math.max(10, Math.round((r.heat / max) * 100)) };
      })
    });
  },

  onShow: function () {
    this.setData({ favorites: getApp().getFavorites() });
  },

  onReady: function () {
    this.drawChart();
  },

  buildCalendar: function () {
    var app = getApp();
    var c = app.getCheckin();
    var now = new Date();
    var year = now.getFullYear();
    var month = now.getMonth();
    var today = now.getDate();
    var first = new Date(year, month, 1).getDay();      // 0=周日
    var lead = (first + 6) % 7;                          // 以周一为首列
    var days = new Date(year, month + 1, 0).getDate();
    var cells = [];
    for (var i = 0; i < lead; i++) {
      cells.push({ key: 'e' + i, day: 0, done: false, today: false });
    }
    for (var d = 1; d <= days; d++) {
      cells.push({
        key: 'd' + d,
        day: d,
        done: c.days.indexOf(d) >= 0,
        today: d === today
      });
    }
    this.setData({
      calendar: cells,
      streak: c.streak || 0,
      checkedToday: c.days.indexOf(today) >= 0,
      nextReward: 5 + Math.min(15, (c.streak || 0))
    });
  },

  computeStats: function () {
    var total = 0;
    this.data.readMinutes.forEach(function (m) { total += m; });
    this.setData({
      weekTotal: total,
      weekAvg: Math.round(total / this.data.readMinutes.length),
      goalPercent: Math.min(100, Math.round((total / 300) * 100))
    });
  },

  drawChart: function () {
    var ctx = wx.createCanvasContext('readChart');
    var data = this.data.readMinutes;
    var labels = ['一', '二', '三', '四', '五', '六', '日'];
    var base = 116, bw = 22, gap = 18;
    ctx.setFillStyle('#FFFFFF');
    ctx.fillRect(0, 0, 320, 140);
    ctx.setStrokeStyle('#F0F0F0');
    ctx.setLineWidth(1);
    for (var g = 0; g <= 3; g++) {
      var gy = 20 + g * 32;
      ctx.beginPath();
      ctx.moveTo(12, gy);
      ctx.lineTo(308, gy);
      ctx.stroke();
    }
    var max = 60;
    for (var i = 0; i < data.length; i++) {
      var h = (data[i] / max) * 92;
      var x = 26 + i * (bw + gap);
      ctx.setFillStyle(data[i] >= 45 ? '#D43C33' : '#F0A5A0');
      ctx.fillRect(x, base - h, bw, h);
      ctx.setFillStyle('#8A8A8A');
      ctx.setFontSize(10);
      ctx.fillText(labels[i], x + 5, base + 14);
      ctx.setFillStyle('#BFBFBF');
      ctx.fillText('' + data[i], x + 2, base - h - 4);
    }
    ctx.draw();
  },

  onCheckin: function () {
    var today = new Date().getDate();
    var ok = getApp().doCheckin(today);
    this.buildCalendar();
    wx.showToast({ title: ok ? '签到成功 +' + this.data.nextReward + ' 积分' : '今天已签到', icon: ok ? 'success' : 'none' });
  },

  onPickDay: function (e) {
    var d = e.currentTarget.dataset.d;
    wx.showToast({ title: '查看 ' + d + ' 日阅读记录', icon: 'none' });
  },

  onRange: function (e) {
    this.applyRange(e.currentTarget.dataset.k);
  },

  onOpen: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onClearFav: function () {
    var self = this;
    wx.showModal({
      title: '清空收藏',
      content: '确定清空全部收藏内容吗？',
      success: function (res) {
        if (res.confirm) {
          wx.setStorageSync('favorites', []);
          self.setData({ favorites: [] });
          wx.showToast({ title: '已清空', icon: 'none' });
        }
      }
    });
  },

  onShare: function () {
    wx.showToast({ title: '已复制热榜链接', icon: 'none' });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
