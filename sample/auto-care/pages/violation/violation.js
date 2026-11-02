const db = require('../../utils/db.js');

// 杭州常见违章（处罚标准按 2022 年公安部令第 163 号记分规则）
const LIB = [
  { act: '驾驶机动车违反道路交通信号灯通行', place: '文三路与学院路交叉口', fine: 200, points: 6 },
  { act: '机动车违反禁止标线指示', place: '中河高架上塘路下匝道', fine: 200, points: 3 },
  { act: '机动车违反规定停放、临时停车且驾驶人不在现场', place: '武林路 128 号门前', fine: 150, points: 0 },
  { act: '驾驶中型以下载客汽车超过规定时速 10% 以上未达 20%', place: '秋石高架（德胜路往南）', fine: 200, points: 1 },
  { act: '不按导向车道行驶', place: '天目山路与教工路交叉口', fine: 200, points: 2 },
  { act: '驾驶机动车有拨打、接听手持电话等妨碍安全驾驶的行为', place: '江南大道与长河路交叉口', fine: 200, points: 3 }
];
const DATES = ['2026-10-28 08:42', '2026-10-15 18:07', '2026-09-30 12:31', '2026-09-03 21:16'];

// 同一车牌每次查到的结果一致：按车牌字符算一个种子
function query(plate) {
  let h = 0;
  for (let i = 0; i < plate.length; i++) h = (h * 131 + plate.charCodeAt(i)) % 1000003;
  const n = h % 4;   // 0~3 条
  const out = [];
  for (let i = 0; i < n; i++) {
    const v = LIB[(h + i * 7) % LIB.length];
    out.push(Object.assign({ id: plate + '-' + i, time: DATES[i] }, v));
  }
  return out;
}

Page({
  data: { cars: [], idx: 0, list: [], loading: false, queried: false, sum: { n: 0, fine: 0, points: 0 } },
  onLoad(q) { this.carId = q.carId || ''; },
  onShow() {
    const cars = db.getCars();
    let idx = cars.findIndex((c) => c.id === this.carId);
    if (idx < 0) idx = Math.max(0, cars.findIndex((c) => c.isDefault));
    this.setData({ cars, idx });
    if (cars.length) this.search();
  },
  onCar(e) { this.setData({ idx: +e.currentTarget.dataset.i }); this.carId = this.data.cars[this.data.idx].id; this.search(); },
  search() {
    const car = this.data.cars[this.data.idx];
    this.setData({ loading: true });
    setTimeout(() => {
      const paid = db.getPaidFines();
      const list = query(car.plate).map((v) => Object.assign(v, { paid: !!paid[v.id], paidAt: paid[v.id] || '' }));
      const todo = list.filter((v) => !v.paid);
      this.setData({
        list, loading: false, queried: true,
        sum: { n: todo.length, fine: todo.reduce((s, v) => s + v.fine, 0), points: todo.reduce((s, v) => s + v.points, 0) }
      });
    }, 400);
  },
  // 缴款：拉起支付失败（桌面端 / 未开通）时提示去交管 12123，用户确认已缴后标记
  pay(e) {
    const v = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    wx.requestPayment({
      timeStamp: String(Date.now()), nonceStr: 'ac' + Date.now(), package: 'prepay_id=fine_' + v.id, signType: 'RSA', paySign: 'demo',
      success: () => this.markPaid(v),
      fail: () => {
        wx.showModal({
          title: '暂不能在线缴款', content: '罚款 ¥' + v.fine + (v.points ? '、记 ' + v.points + ' 分' : '') + '，可在「交管 12123」App 处理。已经处理完了吗？',
          confirmText: '已处理', cancelText: '稍后',
          success: (r) => { if (r.confirm) this.markPaid(v); }
        });
      }
    });
  },
  markPaid(v) {
    db.payFine(v.id);
    wx.showToast({ title: '已标记为已处理', icon: 'success' });
    this.search();
  },
  addCar() { wx.navigateTo({ url: '/pages/car-edit/car-edit' }); }
});
