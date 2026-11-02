// 数据在 utils/data.js，状态（票、券、会员卡、影评）在 utils/store.js；这里留着老接口给页面用
const D = require('./utils/data.js');
const store = require('./utils/store.js');
App({
  MOVIES: D.MOVIES,
  CINEMAS: D.CINEMAS,
  shows: D.shows,
  getMovie: D.getMovie,
  getCinema: D.getCinema,
  getTickets() { return store.tickets(); }
});
