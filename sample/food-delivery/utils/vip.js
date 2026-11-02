// 开通 / 续费超级会员（首页横幅、「我的」会员卡共用）：付款成功才发红包、记会员期
const store = require('./store.js');
const host = require('./host.js');

// resolve(true) 开通成功
function buy() {
  const v = store.getVip();
  const desc = `饱了么超级会员 · ${v.ever ? '续费' : '首月'} 1 个月（立得 4 张 ¥5 无门槛红包）`;
  return host.pay(v.price, desc).then(ok => {
    if (!ok) {
      wx.showToast({ title: '已取消支付', icon: 'none' });
      return false;
    }
    const nv = store.openVip();
    wx.showToast({ title: `${v.active ? '续费' : '开通'}成功，4 张红包已到账`, icon: 'none' });
    return nv.active;
  });
}

module.exports = { buy };
