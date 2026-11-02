// 跨页数据：收藏、路线、实况截图、足迹，都落 Storage（退出重进还在）
const KEYS = { fav: 'cw_fav', route: 'cw_route', shots: 'cw_shots', trail: 'cw_trail' };

function get(k, def) {
  const v = wx.getStorageSync(KEYS[k]);
  return v === '' || v === undefined || v === null ? def : v;
}

function set(k, v) {
  wx.setStorageSync(KEYS[k], v);
}

function toggleFav(id) {
  const fav = get('fav', []);
  const i = fav.indexOf(id);
  if (i >= 0) fav.splice(i, 1); else fav.unshift(id);
  set('fav', fav);
  return i < 0;
}

function addToRoute(id) {
  const r = get('route', []);
  if (r.indexOf(id) >= 0) return false;
  r.push(id);
  set('route', r);
  return true;
}

function visit(id) {
  const t = get('trail', []).filter((x) => x.id !== id);
  const d = new Date();
  t.unshift({ id: id, at: (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + d.getHours() + ':' + ('0' + d.getMinutes()).slice(-2) });
  set('trail', t.slice(0, 20));
}

module.exports = { get, set, toggleFav, addToRoute, visit };
