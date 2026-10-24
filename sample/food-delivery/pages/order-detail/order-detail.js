const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

const MIN = 60000;

Page({
  data: {
    order: { address: {}, items: [] },
    shop: {},
    steps: [],
    progress: 0,
    statusTitle: '',
    statusSub: '',
    createdText: ''
  },

  onLoad(options) {
    this.orderId = options.id || '';
    this.load();
  },

  load() {
    const list = cart.getOrders();
    const order = list.find(o => o.id === this.orderId) ||
      list.find(o => o.status === 'delivering') || list[0];
    this.orderId = order.id;
    const shop = data.findShop(order.shopId);
    const t = order.createdAt;
    const now = Date.now();
    const finished = order.status !== 'delivering';
    const raw = [
      { name: '订单已提交', ts: t, desc: '已通过微信支付 ¥' + order.pay },
      { name: '商家已接单', ts: t + MIN, desc: `${shop.name} 正在备餐` },
      { name: '骑手已到店', ts: t + 9 * MIN, desc: '张师傅已到达商家，等待出餐' },
      { name: '骑手已取餐', ts: t + 12 * MIN, desc: '正在赶往你的位置，距你 1.1km' },
      { name: '已送达', ts: t + 30 * MIN, desc: order.status === 'refund' ? '订单已退款，¥' + order.pay + ' 原路退回' : '放在了 ' + order.address.detail }
    ];
    let lastDone = 0;
    const steps = raw.map((s, i) => {
      const done = finished || s.ts <= now;
      if (done) lastDone = i;
      return { name: s.name, desc: s.desc, done, time: done ? cart.hm(s.ts) : '预计 ' + cart.hm(s.ts) };
    });
    steps[lastDone].current = !finished;
    const progress = finished ? 100 : [12, 22, 34, 62, 100][lastDone];
    let statusTitle = '骑手正在送餐';
    let statusSub = `预计 ${cart.hm(t + 30 * MIN)} 送达 · 准时宝保障中`;
    if (lastDone < 3 && !finished) statusTitle = lastDone < 2 ? '商家正在备餐' : '骑手已到店';
    if (order.status === 'done') { statusTitle = '订单已送达'; statusSub = '感谢使用饱了么，期待再次光临'; }
    if (order.status === 'refund') { statusTitle = '订单已退款'; statusSub = '退款已原路返回，1~3 个工作日到账'; }
    wx.setNavigationBarTitle({ title: finished ? '订单详情' : '配送中' });
    this.setData({ order, shop, steps, progress, statusTitle, statusSub, createdText: cart.fmtTime(t) });
  },

  onCall() {
    wx.showModal({ title: '联系骑手', content: '将通过隐私号 170****8812 呼叫骑手张师傅', confirmText: '呼叫' });
  },

  onCopy() {
    wx.setClipboardData({ data: this.data.order.id });
  },

  onService() {
    wx.showActionSheet({
      itemList: ['催单', '修改订单信息', '申请退款', '在线客服'],
      success: (res) => {
        const tips = ['已为你催促商家和骑手', '骑手已取餐，暂不支持修改', '已提交退款申请', '客服小饱正在赶来'];
        wx.showToast({ title: tips[res.tapIndex], icon: 'none' });
      }
    });
  },

  onConfirm() {
    wx.showModal({
      title: '确认收货',
      content: '确认已经收到餐品了吗？',
      success: (res) => {
        if (!res.confirm) return;
        cart.updateOrder(this.orderId, { status: 'done' });
        wx.showToast({ title: '已确认收货', icon: 'success' });
        this.load();
      }
    });
  },

  onAgain() {
    wx.navigateTo({ url: `/pages/shop/shop?id=${this.data.order.shopId}` });
  }
});
