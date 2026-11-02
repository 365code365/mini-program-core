const db = require('./utils/db.js');

App({
  globalData: { orderTab: '' },
  onLaunch() { db.seed(); }
});
