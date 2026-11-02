// 订单卡片。
// - observers 多字段：'order.status, order.amount' 重算状态文案、颜色、按钮；
//   通配 'order.**'：订单里任何字段变了（改约、换地址）都重算上门倒计时。
// - lifetimes：attached 起计时器、detached 清掉；pageLifetimes：页面隐藏时停表、显示时恢复。
// - 按钮 triggerEvent('action', { type, id }, { bubbles, composed })：订单列表在外层 view 上统一处理。
const STATUS = {
  waiting: { text: '待上门', color: '#13a86b' },
  serving: { text: '服务中', color: '#1e88e5' },
  review: { text: '待评价', color: '#f08c00' },
  done: { text: '已完成', color: '#8b958f' },
  cancelled: { text: '已取消', color: '#b3bab6' }
};
const ACTIONS = {
  waiting: [{ type: 'cancel', text: '取消订单' }, { type: 'call', text: '联系师傅', primary: true }],
  serving: [{ type: 'call', text: '联系师傅' }, { type: 'finish', text: '确认完成', primary: true }],
  review: [{ type: 'again', text: '再来一单' }, { type: 'review', text: '去评价', primary: true }],
  done: [{ type: 'again', text: '再来一单', primary: true }],
  cancelled: [{ type: 'delete', text: '删除订单' }, { type: 'again', text: '再来一单', primary: true }]
};

function leftText(o) {
  if (!o || o.status !== 'waiting') return '';
  const p = String(o.date).split('-'), t = String(o.time).split(':');
  const at = new Date(+p[0], +p[1] - 1, +p[2], +t[0], +t[1]);
  const m = Math.round((at - Date.now()) / 60000);
  if (m <= 0) return '师傅正在赶来';
  if (m < 60) return '距上门 ' + m + ' 分钟';
  if (m < 1440) return '距上门 ' + Math.floor(m / 60) + ' 小时 ' + (m % 60) + ' 分';
  return '距上门 ' + Math.floor(m / 1440) + ' 天';
}

Component({
  properties: { order: { type: Object, value: null } },
  data: { statusText: '', color: '', actions: [], left: '' },
  observers: {
    'order.status, order.amount': function (status, amount) {
      const s = STATUS[status] || STATUS.waiting;
      this.setData({ statusText: s.text, color: s.color, actions: ACTIONS[status] || [], payText: '¥' + (amount || 0) });
    },
    'order.**': function (o) {
      const left = leftText(o);
      if (left !== this.data.left) this.setData({ left });
    }
  },
  lifetimes: {
    attached() { this.start(); },
    detached() { this.stop(); }
  },
  pageLifetimes: {
    show() { this.tick(); this.start(); },
    hide() { this.stop(); }
  },
  methods: {
    start() {
      this.stop();
      if (this.data.order && this.data.order.status === 'waiting') this.timer = setInterval(() => this.tick(), 30000);
    },
    stop() { if (this.timer) { clearInterval(this.timer); this.timer = null; } },
    tick() { const left = leftText(this.data.order); if (left !== this.data.left) this.setData({ left }); },
    onAction(e) {
      this.triggerEvent('action', { type: e.currentTarget.dataset.type, id: this.data.order.id }, { bubbles: true, composed: true });
    },
    onOpen() { this.triggerEvent('action', { type: 'open', id: this.data.order.id }, { bubbles: true, composed: true }); }
  }
});
