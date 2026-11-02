// 订单上的操作（订单列表与详情页共用）：继续支付、取消（退款）、确认收货、再来一单 / 再次预约
const cart = require('./cart.js');
const store = require('./store.js');
const host = require('./host.js');

function pay(o) {
  return host.pay(o.pay, '萌宠家商城 · 共 ' + o.count + ' 件商品').then(ok => {
    if (!ok) return false;
    store.payOrder(o.id);
    wx.showToast({ title: '支付成功，仓库马上发货', icon: 'none' });
    return true;
  });
}

function cancel(o) {
  return new Promise(resolve => {
    if (!store.canCancel(o)) {
      const msg = o.kind === 'service' ? '服务开始前 2 小时内不能在线取消，如需改约请致电门店' : '商品已发货，请收货后联系客服办理退货';
      wx.showModal({ title: '无法取消', content: msg, showCancel: false, confirmText: '知道了' });
      resolve(false);
      return;
    }
    let back = '';
    if (o.kind === 'service') back = [o.pay ? '¥' + o.pay + ' 原路退回' : '', o.cardUsed ? '洗护卡次数退回' : ''].filter(Boolean).join('，');
    else if (o.status === 'paid') back = '¥' + o.pay + ' 原路退回';
    wx.showModal({
      title: o.kind === 'service' ? '取消预约' : '取消订单',
      content: (o.kind === 'service' ? '确定取消这次预约吗？' : '确定取消这笔订单吗？') + (back ? '\n' + back : ''),
      confirmText: '确定取消',
      cancelText: '再想想',
      confirmColor: '#ff5b35',
      success: r => {
        if (!r.confirm) return resolve(false);
        store.cancelOrder(o.id);
        wx.showToast({ title: back ? '已取消，' + back : '订单已取消', icon: 'none' });
        resolve(true);
      }
    });
  });
}

function confirm(o) {
  return new Promise(resolve => {
    wx.showModal({
      title: '确认收货',
      content: '请确认已收到全部 ' + o.count + ' 件商品',
      confirmText: '确认收货',
      success: r => {
        if (!r.confirm) return resolve(false);
        store.confirmReceipt(o.id);
        wx.showToast({ title: '已确认收货，来评价一下吧', icon: 'none' });
        resolve(true);
      }
    });
  });
}

// 再来一单：商品按原规格放回购物车；服务回到同一门店、同一只宠物的预约页
function again(o) {
  if (o.kind === 'goods') {
    o.items.forEach(x => cart.add(x.id, x.spec, x.qty));
    wx.navigateTo({ url: '/pages/cart/cart' });
    return;
  }
  const svc = o.svcs && o.svcs[0] ? o.svcs[0].id : '';
  wx.navigateTo({ url: '/pages/booking/booking?cat=' + o.cat + '&svc=' + svc + '&store=' + o.storeId + '&pet=' + o.petId });
}

module.exports = { pay, cancel, confirm, again };
