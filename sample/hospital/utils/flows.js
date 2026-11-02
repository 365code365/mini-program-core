// 挂号单上的两个操作（挂号单列表与详情页共用）：到院签到、取消预约
const store = require('./store.js');
const host = require('./host.js');

// 签到：只能在就诊当天；能定位就先看是不是在医院附近，定位不了（宿主没有定位能力）就让用户手动确认已到院
function checkin(o) {
  return new Promise(resolve => {
    if (o.date !== store.today()) {
      wx.showToast({ title: '请于就诊当天（' + o.date.slice(5) + '）到院后签到', icon: 'none' });
      resolve(false);
      return;
    }
    host.locate().then(loc => {
      if (loc && loc.dist > 1000) {
        wx.showModal({ title: '不在医院范围内', content: '您距离医院约 ' + (loc.dist / 1000).toFixed(1) + 'km，请到院后再签到', showCancel: false, confirmText: '知道了' });
        resolve(false);
        return;
      }
      wx.showModal({
        title: '到院签到',
        content: o.patient + ' · ' + o.dept + ' ' + o.doctor + '\n请确认已到达 ' + o.room + ' 候诊区',
        confirmText: '确认签到',
        success: r => {
          if (r.confirm) {
            store.checkin(o.id);
            wx.showToast({ title: '签到成功，请候诊', icon: 'none' });
          }
          resolve(!!r.confirm);
        }
      });
    });
  });
}

// 取消：就诊前一天 17:00 前可免费退号，挂号费原路退回；过了截止时间只能到人工窗口
function cancel(o) {
  return new Promise(resolve => {
    if (!store.stageOf(o).canCancel) {
      wx.showModal({ title: '无法在线退号', content: '已过免费退号时间（就诊前一天 17:00），如需退号请到门诊 1 楼人工服务窗口办理', showCancel: false, confirmText: '知道了' });
      resolve(false);
      return;
    }
    wx.showModal({
      title: '取消预约',
      content: '取消后挂号费 ¥' + o.fee + ' 将原路退回，确定取消吗？',
      confirmText: '确定取消',
      confirmColor: '#ff5a36',
      success: r => {
        if (!r.confirm) return resolve(false);
        store.cancelOrder(o.id);
        wx.showToast({ title: '已取消，退款 1~3 个工作日到账', icon: 'none' });
        resolve(true);
      }
    });
  });
}

module.exports = { checkin, cancel };
