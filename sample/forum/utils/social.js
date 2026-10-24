// 点赞 / 收藏的乐观更新：界面先变，0.5 秒后「服务端」确认才写 storage；
// 同一帖子 2 秒内点了 4 次以上就被限流，界面回滚到服务端状态并提示
const store = require('./store.js');
const hits = {};
const pending = {};

function serverHas(kind, id) {
  const set = kind === 'like' ? store.likes() : store.favs();
  return set.indexOf(id) >= 0;
}

function commit(kind, id, on) {
  const key = kind === 'like' ? 'forum_likes' : 'forum_favs';
  let set = kind === 'like' ? store.likes() : store.favs();
  set = set.filter((x) => x !== id);
  if (on) set.unshift(id);
  wx.setStorageSync(key, set);
}

// 在页面的某个列表（data[listKey]）里找到这条帖子并就地改；列表里没有就跳过
function patch(page, listKey, id, kind, on) {
  const list = page.data[listKey] || [];
  const i = list.findIndex((x) => x.id === id);
  if (i < 0) return;
  const it = list[i];
  const d = {};
  if (kind === 'like') {
    if (it.liked === on) return;
    d[listKey + '[' + i + '].liked'] = on;
    d[listKey + '[' + i + '].likes'] = it.likes + (on ? 1 : -1);
  } else {
    d[listKey + '[' + i + '].fav'] = on;
  }
  page.setData(d);
}

function toggle(page, listKey, kind, id) {
  const list = page.data[listKey] || [];
  const it = list.find((x) => x.id === id);
  if (!it) return;
  const on = kind === 'like' ? !it.liked : !it.fav;
  patch(page, listKey, id, kind, on);
  if (kind === 'fav') wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  const k = kind + id;
  const now = Date.now();
  hits[k] = (hits[k] || []).filter((t) => now - t < 2000).concat([now]);
  clearTimeout(pending[k]);
  pending[k] = setTimeout(() => {
    if (hits[k].length > 4) {
      hits[k] = [];
      patch(page, listKey, id, kind, serverHas(kind, id));
      wx.showToast({ title: '操作太频繁，请稍后再试', icon: 'none' });
      return;
    }
    commit(kind, id, on);
  }, 500);
}

module.exports = { toggle };
