// 我的订单：状态筛选 / 物流时间轴 / 确认收货
var STATUS_TEXT = { pending: '待付款', paid: '待发货', shipping: '运输中', done: '已完成' };

Page({
  data: {
    statusTabs: [],
    statusKey: 'all',
    list: [],
    openId: '',
    all: []
  },

  onLoad: function (options) {
    this.reload();
    if (options && options.new) {
      this.setData({ openId: options.new });
      wx.showToast({ title: '订单已创建', icon: 'success' });
    }
  },

  onShow: function () {
    this.reload();
  },

  reload: function () {
    var app = getApp();
    var orders = app.getOrders();
    if (orders.length === 0) {
      orders = this.demoOrders();
    }
    var decorated = orders.map(function (o, i) {
      var progress = o.status === 'done' ? 100 : (o.status === 'shipping' ? 65 : (o.status === 'paid' ? 30 : 5));
      return {
        id: o.id,
        items: o.items || [],
        total: o.total,
        ship: o.ship || '普通快递',
        createdAt: o.createdAt,
        status: o.status,
        statusText: STATUS_TEXT[o.status] || '处理中',
        progress: progress,
        trace: [
          { time: o.createdAt, text: progress >= 100 ? '快件已签收，感谢使用' : (progress >= 65 ? '快件已到达【深圳南山营业点】' : '商家已接单，正在打包') },
          { time: o.createdAt, text: '订单支付成功，等待商家发货' },
          { time: o.createdAt, text: '订单提交成功' }
        ],
        _i: i
      };
    });
    var counts = { all: decorated.length, pending: 0, paid: 0, shipping: 0, done: 0 };
    decorated.forEach(function (o) { counts[o.status] = (counts[o.status] || 0) + 1; });
    this.setData({
      all: decorated,
      statusTabs: [
        { key: 'all', name: '全部', count: counts.all },
        { key: 'pending', name: '待付款', count: counts.pending || 0 },
        { key: 'paid', name: '待发货', count: counts.paid || 0 },
        { key: 'shipping', name: '运输中', count: counts.shipping || 0 },
        { key: 'done', name: '已完成', count: counts.done || 0 }
      ]
    });
    this.applyFilter();
  },

  demoOrders: function () {
    return [
      {
        id: 'SO20260712001', createdAt: '2026-07-12 10:24', status: 'shipping', total: '199.00', ship: '顺丰特快',
        items: [{ id: 101, name: '无线蓝牙耳机 Pro', spec: '黑色', price: 199, quantity: 1, image: '/assets/p_earbuds.jpg' }]
      },
      {
        id: 'SO20260705002', createdAt: '2026-07-05 19:02', status: 'done', total: '688.00', ship: '普通快递',
        items: [
          { id: 102, name: '智能运动手表', spec: '午夜黑', price: 599, quantity: 1, image: '/assets/p_watch.jpg' },
          { id: 105, name: '精品咖啡礼盒', spec: '中深烘焙', price: 89, quantity: 1, image: '/assets/p_coffee.jpg' }
        ]
      }
    ];
  },

  applyFilter: function () {
    var key = this.data.statusKey;
    var list = key === 'all' ? this.data.all : this.data.all.filter(function (o) { return o.status === key; });
    this.setData({ list: list });
  },

  onSwitchStatus: function (e) {
    this.setData({ statusKey: e.currentTarget.dataset.key });
    this.applyFilter();
  },

  onToggleTrace: function (e) {
    var id = e.currentTarget.dataset.id;
    this.setData({ openId: this.data.openId === id ? '' : id });
  },

  onGoodsTap: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onContact: function () {
    wx.showToast({ title: '客服将尽快联系你', icon: 'none' });
  },

  onRemind: function () {
    wx.showToast({ title: '已提醒商家发货', icon: 'success' });
  },

  onConfirm: function (e) {
    var self = this;
    var id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认收货',
      content: '确认已收到商品？确认后订单将完成。',
      success: function (res) {
        if (res.confirm) {
          getApp().updateOrderStatus(id, 'done');
          self.reload();
          wx.showToast({ title: '交易完成', icon: 'success' });
        }
      }
    });
  },

  onReview: function () {
    wx.showToast({ title: '感谢评价 ⭐️⭐️⭐️⭐️⭐️', icon: 'none' });
  },

  onGoHome: function () {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
