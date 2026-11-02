// 本地存储：爱车、订单、违章处理状态、取送车地址。全部走 wx.setStorageSync，退出重进还在。
const { STORES, CAR_IMGS } = require('./data.js');
const util = require('./util.js');

const K = { cars: 'ac_cars', orders: 'ac_orders', fines: 'ac_fines_paid', addr: 'ac_pickup_addr', seeded: 'ac_seeded_v1' };
const CARE_KM = 10000, CARE_MONTHS = 6;

function seed() {
  if (wx.getStorageSync(K.seeded)) return;
  const cars = [
    { id: 'c1', brandId: 'b-vw', brand: '大众', model: '帕萨特', year: '2021 款 330TSI 精英版', color: 'white', plate: '浙A5K21D',
      vin: 'LSVAU2A37M2018365', mileage: 32580, buyDate: '2021-04-18', lastCareDate: '2026-06-12', lastCareKm: 27600, isDefault: true },
    { id: 'c2', brandId: 'b-tesla', brand: '特斯拉', model: 'Model Y', year: '2023 款 后轮驱动版', color: 'blue', plate: '浙AD80196',
      vin: 'LRWYGCEK5PC123457', mileage: 18240, buyDate: '2023-03-02', lastCareDate: '2026-03-20', lastCareKm: 10500, isDefault: false }
  ];
  const s = STORES[0];
  const orders = [{
    id: 'AC20260612093015', storeId: s.id, storeName: s.name, storeAddr: s.address, storePhone: s.phone,
    carId: 'c1', carName: '大众 帕萨特', plate: '浙A5K21D',
    items: [{ id: 'v1', name: '小保养', price: 299, mins: 60 }, { id: 'v15', name: '36 项全车检测', price: 0, mins: 30 }],
    total: 299, date: '2026-06-12', time: '09:30', contact: '陈先生', phone: '13857126608', pickup: false, pickupAddr: '', remark: '',
    status: 'done', createdAt: '2026-06-10 20:15', cancelReason: '', review: null,
    timeline: [
      { t: '2026-06-10 20:15', text: '预约成功，门店已确认工位' },
      { t: '2026-06-12 09:26', text: '到店签到，技师王师傅接车' },
      { t: '2026-06-12 10:41', text: '服务完成，已交车' }
    ]
  }];
  wx.setStorageSync(K.cars, cars);
  wx.setStorageSync(K.orders, orders);
  wx.setStorageSync(K.seeded, 1);
}

// 爱车 + 派生字段（车卡图、下次保养）
function decorate(c) {
  const nextKm = c.lastCareKm + CARE_KM;
  const d = util.parseYmd(c.lastCareDate); d.setMonth(d.getMonth() + CARE_MONTHS);
  const nextDate = util.ymd(d);
  const leftDays = util.daysBetween(util.ymd(new Date()), nextDate);
  const leftKm = nextKm - c.mileage;
  const used = Math.min(100, Math.max(0, Math.round((c.mileage - c.lastCareKm) / CARE_KM * 100)));
  return Object.assign({}, c, {
    name: c.brand + ' ' + c.model, img: CAR_IMGS[c.color] || CAR_IMGS.white, plateText: util.fmtPlate(c.plate),
    nextKm, nextDate, leftKm, leftDays, used, due: leftKm <= 1000 || leftDays <= 15
  });
}

function getCars() { return (wx.getStorageSync(K.cars) || []).map(decorate); }
function saveCars(list) {
  // 存原始字段，派生字段每次读的时候算
  const keys = ['id', 'brandId', 'brand', 'model', 'year', 'color', 'plate', 'vin', 'mileage', 'buyDate', 'lastCareDate', 'lastCareKm', 'isDefault'];
  wx.setStorageSync(K.cars, list.map((c) => { const o = {}; keys.forEach((k) => { o[k] = c[k]; }); return o; }));
}
function getCar(id) { return getCars().find((c) => c.id === id) || null; }
function defaultCar() { const cs = getCars(); return cs.find((c) => c.isDefault) || cs[0] || null; }
function upsertCar(car) {
  const list = getCars();
  const i = list.findIndex((c) => c.id === car.id);
  if (car.isDefault || !list.length) list.forEach((c) => { c.isDefault = false; });
  if (!list.length) car.isDefault = true;
  if (i >= 0) list[i] = car; else list.push(car);
  saveCars(list);
}
function removeCar(id) {
  const list = getCars().filter((c) => c.id !== id);
  if (list.length && !list.some((c) => c.isDefault)) list[0].isDefault = true;
  saveCars(list);
}
function setDefaultCar(id) { saveCars(getCars().map((c) => Object.assign(c, { isDefault: c.id === id }))); }
// 服务完成后把本次保养写回爱车档案
function markCareDone(carId, date) {
  const list = getCars();
  const c = list.find((x) => x.id === carId);
  if (!c) return;
  c.lastCareDate = date; c.lastCareKm = c.mileage;
  saveCars(list);
}

function getOrders() { return wx.getStorageSync(K.orders) || []; }
function getOrder(id) { return getOrders().find((o) => o.id === id) || null; }
function saveOrder(order) {
  const list = getOrders();
  const i = list.findIndex((o) => o.id === order.id);
  if (i >= 0) list[i] = order; else list.unshift(order);
  wx.setStorageSync(K.orders, list);
}

function getPaidFines() { return wx.getStorageSync(K.fines) || {}; }
function payFine(id) { const m = getPaidFines(); m[id] = util.stamp(); wx.setStorageSync(K.fines, m); }

function getPickupAddr() { return wx.getStorageSync(K.addr) || null; }
function setPickupAddr(a) { wx.setStorageSync(K.addr, a); }

module.exports = {
  seed, getCars, getCar, defaultCar, upsertCar, removeCar, setDefaultCar, markCareDone,
  getOrders, getOrder, saveOrder, getPaidFines, payFine, getPickupAddr, setPickupAddr
};
