const store = require('./utils/store.js');
App({
  onLaunch() { store.ensure(); }
});
