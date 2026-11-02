const db = require('./utils/db.js');

App({
  globalData: { serviceCat: '' },
  onLaunch() {
    db.seed();
  }
});
