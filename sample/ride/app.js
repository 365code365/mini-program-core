const CAR_TYPES = [
  { key: 'express', name: '快车', desc: '经济实惠', base: 10, perKm: 1.8, perMin: 0.45, seats: 4 },
  { key: 'premier', name: '专车', desc: '舒适商务', base: 16, perKm: 3.2, perMin: 0.8, seats: 4 },
  { key: 'pool', name: '拼车', desc: '一口价 省钱', base: 6, perKm: 1.2, perMin: 0.3, seats: 2 }
];

const SEED_TRIPS = [
  { id: 'T20261023A1', type: '快车', from: '望京SOHO T1-东门', to: '北京首都国际机场T3航站楼', date: '10月23日 08:12', km: 21.4, min: 36, status: 'done', plate: '京B·Q3M81', driver: '李师傅', car: '大众 朗逸 · 银色' },
  { id: 'T20261020B7', type: '专车', from: '国贸三期', to: '三里屯太古里北区', date: '10月20日 19:46', km: 5.8, min: 22, status: 'done', plate: '京A·N0P66', driver: '赵师傅', car: '奥迪 A6L · 黑色' },
  { id: 'T20261018C2', type: '拼车', from: '中关村软件园二期', to: '西二旗地铁站', date: '10月18日 18:30', km: 4.1, min: 15, status: 'cancel', plate: '-', driver: '-', car: '-' },
  { id: 'T20261012D9', type: '快车', from: '北京南站', to: '五道口购物中心', date: '10月12日 21:05', km: 23.7, min: 48, status: 'done', plate: '京Q·7H2C5', driver: '陈师傅', car: '丰田 卡罗拉 · 白色' }
];

function estimate(type, km, min) {
  const t = CAR_TYPES.find(c => c.key === type) || CAR_TYPES[0];
  let price = t.base + t.perKm * km + t.perMin * min;
  if (type === 'pool') price = price * 0.82;
  return Math.round(price * 10) / 10;
}

function fees(trip) {
  const key = trip.type === '专车' ? 'premier' : trip.type === '拼车' ? 'pool' : 'express';
  const t = CAR_TYPES.find(c => c.key === key);
  const mileage = Math.round(t.perKm * trip.km * 10) / 10;
  const time = Math.round(t.perMin * trip.min * 10) / 10;
  const far = trip.km > 15 ? Math.round((trip.km - 15) * 0.8 * 10) / 10 : 0;
  const coupon = trip.km > 10 ? 5 : 0;
  const total = Math.round((t.base + mileage + time + far - coupon) * 10) / 10;
  return { base: t.base, mileage, time, far, coupon, total };
}

App({
  globalData: { CAR_TYPES },
  estimate,
  fees,
  getTrips() {
    const mine = wx.getStorageSync('ride_trips') || [];
    return mine.concat(SEED_TRIPS);
  },
  addTrip(trip) {
    const mine = wx.getStorageSync('ride_trips') || [];
    mine.unshift(trip);
    wx.setStorageSync('ride_trips', mine.slice(0, 20));
  }
});
