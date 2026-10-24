const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

const MIN = 60000;

Page({
  data: {
    order: null,
    store: data.stores[0],
    steps: [],
    tip: '',
    history: []
  },

  onShow() {
    this.load();
    // 制作进度按时间推进：停在页面上也会自己更新
    clearInterval(this.timer);
    this.timer = setInterval(() => this.load(), 15000);
  },

  onHide() {
    clearInterval(this.timer);
  },

  onUnload() {
    clearInterval(this.timer);
  },

  load() {
    const orders = store.getOrders();
    if (!orders.length) {
      this.setData({ order: null });
      return;
    }
    const order = orders[0];
    const elapsed = Date.now() - order.createdAt;
    const pickup = order.mode === 'pickup';
    const names = pickup ? ['已下单', '制作中', '待取餐', '已完成'] : ['已下单', '制作中', '配送中', '已送达'];
    let stage = 1;
    if (elapsed > 4 * MIN) stage = 2;
    if (elapsed > (pickup ? 30 : 35) * MIN) stage = 3;
    const tips = pickup
      ? ['', '咖啡师正在为你制作，预计 3 分钟后可取', '已制作完成，请凭取餐码到吧台取餐', '订单已完成，欢迎再次光临']
      : ['', '咖啡师正在为你制作', '骑手已取餐，正在飞奔向你', '已送达，祝你享用愉快'];
    const steps = names.map((name, i) => ({ name, done: i <= stage }));
    const history = orders.slice(1, 6).map(o => ({
      id: o.id,
      code: o.code,
      pay: o.pay,
      summary: o.items.map(i => `${i.name}×${i.qty}`).join('、'),
      time: store.dateTime(o.createdAt)
    }));
    this.setData({
      order,
      store: data.stores.find(s => s.id === order.storeId) || data.stores[0],
      steps,
      tip: tips[stage],
      history
    });
  },

  onCall() {
    wx.showModal({ title: '联系门店', content: '门店电话 010-5718 6620', confirmText: '呼叫' });
  },

  onNav() {
    wx.showToast({ title: `步行约 ${this.data.store.distance}`, icon: 'none' });
  },

  onGoMenu() {
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
