// 购物车：[{ id, spec, qty, checked }] 存 storage
function get() { return wx.getStorageSync('pet_cart') || []; }
function save(v) { wx.setStorageSync('pet_cart', v); }
function add(id, spec, qty) {
  const list = get();
  const hit = list.find(x => x.id === id && x.spec === spec);
  if (hit) hit.qty += qty; else list.push({ id, spec, qty, checked: true });
  save(list);
  return count();
}
function count() { return get().reduce((a, x) => a + x.qty, 0); }
module.exports = { get, save, add, count };
