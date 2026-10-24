// 购物车 / 订单 / 用餐方式：都存在本地
const data = require('./data.js');

const CART = 'cf_cart';
const ORDERS = 'cf_orders';
const MODE = 'cf_mode';

function round2(n) {
  return Math.round(n * 100) / 100;
}

function getMode() {
  return wx.getStorageSync(MODE) || 'pickup';
}

function setMode(m) {
  wx.setStorageSync(MODE, m);
}

function getCart() {
  return wx.getStorageSync(CART) || [];
}

function saveCart(list) {
  wx.setStorageSync(CART, list);
}

// 同一饮品同一规格合并成一行
function addToCart(item) {
  const list = getCart();
  const key = [item.drinkId, item.size, item.temp, item.sugar].join('|');
  const hit = list.find(x => x.key === key);
  if (hit) hit.qty += item.qty;
  else list.push(Object.assign({ key }, item));
  saveCart(list);
  return list;
}

function changeQty(key, delta) {
  const list = getCart();
  const i = list.findIndex(x => x.key === key);
  if (i < 0) return list;
  list[i].qty += delta;
  if (list[i].qty <= 0) list.splice(i, 1);
  saveCart(list);
  return list;
}

function summary() {
  const list = getCart().map(x => {
    const d = data.findDrink(x.drinkId);
    return Object.assign({}, x, { name: d.name, img: d.img, total: round2(x.unit * x.qty) });
  });
  const count = list.reduce((n, x) => n + x.qty, 0);
  const amount = round2(list.reduce((n, x) => n + x.unit * x.qty, 0));
  return { list, count, amount };
}

function getOrders() {
  return wx.getStorageSync(ORDERS) || [];
}

function addOrder(o) {
  const list = getOrders();
  list.unshift(o);
  wx.setStorageSync(ORDERS, list.slice(0, 20));
}

function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}

function hm(ts) {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function dateTime(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${hm(ts)}`;
}

module.exports = { round2, getMode, setMode, getCart, addToCart, changeQty, summary, saveCart, getOrders, addOrder, hm, dateTime };
