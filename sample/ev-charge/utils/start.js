// 开始充电的公共流程（扫码页、电站详情点终端都走这里）：检查 → 引导（充值 / 付欠款 / 加车）→ 建会话 → 进充电页
const store = require('./store.js');
// 能开始返回 true；不能就按原因引导，返回 false
function guard(no, onErr) {
  const r = store.checkStart(no);
  if (!r.err) return true;
  const go = (title, content, confirmText, fn) => wx.showModal({
    title, content, confirmText, confirmColor: '#00B87A',
    success: m => { if (m.confirm) fn(); }
  });
  if (r.kind === 'session') wx.redirectTo({ url: '/pages/charging/charging' });
  else if (r.kind === 'recharge') go('余额不足', r.err, '去充值', () => wx.switchTab({ url: '/pages/wallet/wallet' }));
  else if (r.kind === 'unpaid') go('有未支付订单', r.err, '去支付', () => wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + r.id }));
  else if (r.kind === 'car') go('还没有车辆', '添加车辆后才能充电（用于停车减免与电量估算）', '去添加', () => wx.navigateTo({ url: '/pages/vehicle-edit/vehicle-edit' }));
  else if (onErr) onErr(r.err);
  else wx.showToast({ title: r.err, icon: 'none' });
  return false;
}
// replace：从扫码页进来时替换掉扫码页（扫码页 onShow 见到会话会跳充电页，留在栈里返回时会来回跳）
function begin(no, replace) {
  store.startSession(no);
  const url = '/pages/charging/charging';
  if (replace) wx.redirectTo({ url });
  else wx.navigateTo({ url });
}
// 电站详情点空闲终端：确认插枪后开始
function confirm(no, price) {
  if (!guard(no)) return;
  const hit = store.terminal(no);
  const car = store.defaultCar();
  wx.showModal({
    title: hit.terminal.no + '（' + hit.terminal.power + 'kW）',
    content: '车辆 ' + car.plate + '\n请确认已插好充电枪，当前电价 ¥' + price + '/度',
    confirmText: '开始充电',
    confirmColor: '#00B87A',
    success: r => { if (r.confirm) begin(no); }
  });
}
module.exports = { guard, begin, confirm };
