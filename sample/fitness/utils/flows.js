// 团课上的三个操作（团课列表、课程详情、我的预约共用）：预约（含候补、单次付费）、取消、签到
const data = require('./data.js');
const store = require('./store.js');
const host = require('./host.js');

// resolve(true)：约上了（或进了候补）
function book(c) {
  return new Promise(resolve => {
    const r = store.checkBook(c);
    if (r.err) {
      wx.showToast({ title: r.err, icon: 'none' });
      resolve(false);
      return;
    }
    if (r.needPay) {
      if (c.left <= 0) {
        wx.showToast({ title: '这节已满员，会员才能排候补', icon: 'none' });
        resolve(false);
        return;
      }
      wx.showActionSheet({
        itemList: ['购买单次课 ¥' + data.SINGLE_PRICE, '开通 / 续费会员卡'],
        success: res => {
          if (res.tapIndex === 1) {
            wx.navigateTo({ url: '/pages/card/card' });
            resolve(false);
            return;
          }
          host.pay(data.SINGLE_PRICE, '单次团课 · ' + c.name + '\n' + c.dateText + ' ' + c.time).then(ok => {
            if (ok) {
              store.book(c, data.SINGLE_PRICE);
              wx.showToast({ title: '预约成功，开课前 30 分钟可签到', icon: 'none' });
            }
            resolve(ok);
          });
        },
        fail: () => resolve(false)
      });
      return;
    }
    const b = store.book(c);
    wx.showToast({ title: b.status === 'wait' ? '已排候补第 ' + b.rank + ' 位，有空位自动转为预约' : '预约成功，开课前 30 分钟可签到', icon: 'none' });
    resolve(true);
  });
}

function cancel(b) {
  return new Promise(resolve => {
    if (b.status !== 'booked' && b.status !== 'wait') return resolve(false);
    let content = '';
    if (b.status === 'wait') content = '确定不再排「' + b.name + '」的候补了吗？';
    else if (store.lateFree(b)) content = '开课前 2 小时可免费取消' + (b.paid ? '，¥' + b.paid + ' 原路退回' : '') + '，确定取消「' + b.name + '」吗？';
    else content = '距开课已不足 2 小时，取消将记 1 次爽约（近 30 天已 ' + store.strikes().length + ' 次，满 3 次暂停预约 7 天）' + (b.paid ? '，¥' + b.paid + ' 原路退回' : '');
    wx.showModal({
      title: b.status === 'wait' ? '取消候补' : '取消预约',
      content,
      confirmText: '确定取消',
      cancelText: '再想想',
      confirmColor: '#ff3d6e',
      success: r => {
        if (!r.confirm) return resolve(false);
        const done = store.cancelBooking(b.bid);
        wx.showToast({ title: done ? (done.late ? '已取消，记 1 次爽约' : '已取消') : '已开课，无法取消', icon: 'none' });
        resolve(!!done);
      }
    });
  });
}

// 签到：开课前 30 分钟到下课之间，在门店扫码；宿主没有扫码能力就页面里确认
function sign(b) {
  return new Promise(resolve => {
    if (!store.signState(b)) {
      const t = store.startOf(b) - Date.now();
      wx.showToast({ title: b.date !== store.today() ? '上课当天才能签到' : (t > 0 ? '开课前 30 分钟开始签到' : '这节课已经结束了'), icon: 'none' });
      resolve(false);
      return;
    }
    wx.showModal({
      title: '到店签到',
      content: b.name + ' · ' + b.time + ' ' + b.room + '\n请确认已到达' + b.store,
      confirmText: '确认签到',
      success: r => {
        if (!r.confirm) return resolve(false);
        store.signIn(b.bid);
        wx.showToast({ title: '签到成功，已记入训练记录并打卡', icon: 'none' });
        resolve(true);
      }
    });
  });
}

module.exports = { book, cancel, sign };
