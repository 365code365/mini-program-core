function log(name) {
  return function (e) {
    var t = (e.touches && e.touches[0]) || (e.changedTouches && e.changedTouches[0]) || {};
    console.log('EV ' + e.type + ' -> ' + name
      + ' target=' + (e.target && e.target.dataset ? e.target.dataset.who : '?')
      + ' current=' + (e.currentTarget && e.currentTarget.dataset ? e.currentTarget.dataset.who : '?')
      + ' touches=' + (e.touches ? e.touches.length : 'x')
      + ' changed=' + (e.changedTouches ? e.changedTouches.length : 'x')
      + ' pt=' + t.clientX + ',' + t.clientY
      + ' ts=' + (typeof e.timeStamp));
  };
}
Page({
  data: {},
  capOuterTS: log('capOuterTS'),
  tsOuter: log('tsOuter'),
  tmOuter: log('tmOuter'),
  teOuter: log('teOuter'),
  tapOuter: log('tapOuter'),
  lpOuter: log('lpOuter'),
  tsInner: log('tsInner'),
  tmInner: log('tmInner'),
  teInner: log('teInner'),
  tapInner: log('tapInner'),
  lpInner: log('lpInner'),
  lockMove: log('lockMove'),
  tcOuter: log('tcOuter'),
  tcInner: log('tcInner'),
});
