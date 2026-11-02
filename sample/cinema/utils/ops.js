// 电影票上的操作（「我的票」与订单详情共用）：退票、改签、导航、联系影院
const D = require('./data.js');
const store = require('./store.js');
const host = require('./host.js');
function refund(t, done) {
  if (!t.canRefund) { wx.showToast({ title: '开场前 60 分钟内不可退票', icon: 'none' }); return; }
  const fee = D.REFUND_FEE * t.seats.length;
  wx.showModal({
    title: '确认退票',
    content: `手续费 ¥${D.REFUND_FEE}/张，共 ¥${fee}；退回 ¥${Math.max(0, t.total - fee)} 到${t.payBy === 'card' ? '会员卡' : '微信零钱'}${t.coupon ? '，优惠券退回账户' : ''}`,
    confirmText: '确认退票',
    confirmColor: '#E8243F',
    success: r => {
      if (!r.confirm) return;
      const res = store.refund(t.id);
      if (!res) { wx.showToast({ title: '这张票不能退了', icon: 'none' }); return; }
      wx.showToast({ title: '已退 ¥' + res.amount, icon: 'success' });
      done && done();
    }
  });
}
function change(t, done) {
  if (t.changed) { wx.showToast({ title: '每张票只能改签一次', icon: 'none' }); return; }
  if (!t.canChange) { wx.showToast({ title: '开场前 60 分钟内不可改签', icon: 'none' }); return; }
  const opts = store.changeOptions(t);
  if (!opts.length) { wx.showToast({ title: '没有可改签的场次', icon: 'none' }); return; }
  wx.showActionSheet({
    itemList: opts.map(o => o.label),
    success: r => {
      const s = opts[r.tapIndex];
      const diff = (s.price - t.price) * t.seats.length;
      const target = `${s.dayText} ${s.time}（${s.hall}）`;
      const apply = () => {
        store.changeTicket(t.id, s, diff);
        wx.showToast({ title: '改签成功', icon: 'success' });
        done && done();
      };
      // 要补差价且用微信付：支付确认本身就是那一步确认，不再先弹一个「确认改签」
      if (diff > 0 && t.payBy !== 'card') {
        host.pay(diff, `改签到 ${target}，座位号不变，补差价`).then(ok => ok && apply());
        return;
      }
      if (diff > 0 && store.card().balance < diff) { wx.showToast({ title: '会员卡余额不足', icon: 'none' }); return; }
      const tip = diff > 0 ? `从会员卡扣差价 ¥${diff}` : diff < 0 ? `退回差价 ¥${-diff}` : '无差价';
      wx.showModal({
        title: '确认改签',
        content: `改到 ${target}，座位号不变，${tip}。改签后不能再次改签。`,
        confirmText: '改签',
        success: m => m.confirm && apply()
      });
    }
  });
}
function nav(t) { host.openMap(D.getCinema(t.cid)); }
function call(t) { host.call(D.getCinema(t.cid).phone); }
module.exports = { refund, change, nav, call };
