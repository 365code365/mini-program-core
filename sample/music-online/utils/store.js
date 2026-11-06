// 本地状态：我喜欢的歌、收藏的歌单、搜索历史（最近播放在 utils/player.js 里记）。接口是只读演示数据，这些都只存 storage
const LIKE_KEY = 'music_liked';
const FAV_KEY = 'music_fav_playlists';
const HISTORY_KEY = 'music_search_history';
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : [];
}
// ───────── 我喜欢 ─────────
function liked() { return list(LIKE_KEY); }
function isLiked(id) { return liked().some((x) => x.id === id); }
function toggleLike(song) {
  const l = liked();
  const on = !l.some((x) => x.id === song.id);
  wx.setStorageSync(LIKE_KEY, on ? [song].concat(l) : l.filter((x) => x.id !== song.id));
  return on;
}
// ───────── 收藏歌单：存卡片要的字段，「我的」离线也能画 ─────────
function favPlaylists() { return list(FAV_KEY); }
function isFav(id) { return favPlaylists().some((p) => p.id === id); }
function toggleFav(p) {
  const l = favPlaylists();
  const on = !l.some((x) => x.id === p.id);
  const card = { id: p.id, name: p.name, cover: p.cover, songCount: p.songCount, playCountText: p.playCountText, creator: p.creator ? p.creator.name : '' };
  wx.setStorageSync(FAV_KEY, on ? [card].concat(l) : l.filter((x) => x.id !== p.id));
  return on;
}
// ───────── 搜索历史：最近 10 条，去重 ─────────
function history() { return list(HISTORY_KEY); }
function addHistory(kw) {
  wx.setStorageSync(HISTORY_KEY, [kw].concat(history().filter((x) => x !== kw)).slice(0, 10));
}
function clearHistory() { wx.removeStorageSync(HISTORY_KEY); }
module.exports = { LIKE_KEY, FAV_KEY, liked, isLiked, toggleLike, favPlaylists, isFav, toggleFav, history, addHistory, clearHistory };
