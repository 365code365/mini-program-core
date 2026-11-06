// 本地状态：「想看」清单与自己写的短评。接口是只读演示数据，自己的短评存 storage，页面上与接口拉来的合并展示
const WISH_KEY = 'movie_wish';
const CMT_KEY = 'movie_my_comments';
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : [];
}
function pad(n) { return (n < 10 ? '0' : '') + n; }
function today() {
  const d = new Date();
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}
// ───────── 想看 ─────────
function getWish() {
  return list(WISH_KEY);
}
function isWished(id) {
  return getWish().some((m) => m.id === id);
}
// 存卡片要的几个字段而不是只存 id：「我的」页离线也能画出来，不用再逐个请求详情
function toggleWish(movie) {
  let l = getWish();
  const on = !l.some((m) => m.id === movie.id);
  if (on) {
    l = [{ id: movie.id, title: movie.title, poster: movie.poster, score: movie.score, genreText: movie.genreText,
      releaseDate: movie.releaseDate, status: movie.status, addedAt: Date.now() }].concat(l);
  } else {
    l = l.filter((m) => m.id !== movie.id);
  }
  wx.setStorageSync(WISH_KEY, l);
  return on;
}
function clearWish() {
  wx.removeStorageSync(WISH_KEY);
}
// ───────── 我的短评：每部影片一条，再写就是修改 ─────────
function myComments() {
  return list(CMT_KEY);
}
function myCommentOf(movieId) {
  return myComments().find((c) => c.movieId === movieId) || null;
}
// 与接口短评同一个形状（user / star / content / date / likes），列表模板不用分两套
function saveComment(movie, star, content) {
  const l = myComments();
  const old = l.find((c) => c.movieId === movie.id);
  const c = Object.assign({ id: 'my-' + Date.now(), likes: 0 }, old || {}, {
    movieId: movie.id, movieTitle: movie.title, poster: movie.poster,
    star, content, date: today(), ts: Date.now(), mine: true, user: { name: '我', avatar: '' }
  });
  wx.setStorageSync(CMT_KEY, [c].concat(l.filter((x) => x.movieId !== movie.id)));
  return c;
}
function removeComment(id) {
  wx.setStorageSync(CMT_KEY, myComments().filter((c) => c.id !== id));
}
// 接口短评前面放上自己的那条（去重：同 id 不重复）
function mergeComments(movieId, apiList) {
  const mine = myCommentOf(movieId);
  const rest = (apiList || []).filter((c) => !mine || c.id !== mine.id);
  return mine ? [mine].concat(rest) : rest;
}
module.exports = { getWish, isWished, toggleWish, clearWish, myComments, myCommentOf, saveComment, removeComment, mergeComments };
