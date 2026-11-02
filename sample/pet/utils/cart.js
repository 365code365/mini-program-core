// 购物车：[{ id, spec, qty, checked }] 存 storage
// 第一次打开放两件示例商品（演示用）。放在这里而不是购物车页的 onShow：
// 否则没进过购物车页之前，「我的」与商品详情的角标是 0，一进购物车却有 3 件，前后对不上
function seed() {
  if (wx.getStorageSync('pet_cart_seeded')) return;
  wx.setStorageSync('pet_cart_seeded', true);
  const v = wx.getStorageSync('pet_cart');
  if (Array.isArray(v) && v.length) return;
  wx.setStorageSync('pet_cart', [
    { id: 'g1', spec: '1.5kg', qty: 1, checked: true },
    { id: 'g3', spec: '套装 A', qty: 2, checked: true }
  ]);
}
function get() {
  seed();
  const v = wx.getStorageSync('pet_cart');
  return Array.isArray(v) ? v : [];
}
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
