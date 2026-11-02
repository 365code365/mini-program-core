// 激励视频复活 + 转发。两样都要宿主（微信后台）：
// - wx.createRewardedVideoAd：建出来就拉一次广告，onError（无广告 / 没接宿主）之后 show() 也会 reject ——
//   这时**直接给复活**（按钮写明「广告暂不可用，免费复活」），不能让玩家卡在结算页；onClose 里 isEnded 为 true 才发奖励。
// - wx.showShareMenu 打开右上角菜单的转发 / 朋友圈；wx.onShareAppMessage 给菜单转发的标题与 query；
//   主动转发（结算页「炫耀一下」）调 wx.shareAppMessage。官方的 shareAppMessage 没有回调，这里多带一个 fail
//   只为在没宿主的环境里提示一声（微信里不会触发）。
var AD_UNIT = 'adunit-6c1f0b9e4d27a358';

function make(getShare) {
  var ad = null, adState = 'none', pending = null, stats = { adShows: 0, adErrors: 0, freeRevives: 0, adRevives: 0, shares: 0, shareFails: 0 };
  function toast(t) { try { wx.showToast && wx.showToast({ title: t, icon: 'none', duration: 1600 }); } catch (e) {} }
  if (wx.createRewardedVideoAd) {
    try {
      ad = wx.createRewardedVideoAd({ adUnitId: AD_UNIT });
      adState = 'loading';
      ad.onLoad && ad.onLoad(function () { adState = 'ready'; });
      ad.onError && ad.onError(function (e) {
        adState = 'error'; stats.adErrors++;
        console.log('[bubble] 激励视频不可用：' + (e && e.errMsg) + '（' + (e && e.errCode) + '），复活改成免费');
      });
      ad.onClose && ad.onClose(function (res) {
        var cb = pending; pending = null;
        if (!cb) return;
        // 基础库 2.1.0 以下 res 是 undefined，按看完处理
        if (!res || res.isEnded) { stats.adRevives++; cb('ad'); } else { toast('看完广告才能复活哦'); cb(null); }
      });
    } catch (e) { ad = null; adState = 'error'; }
  } else adState = 'error';

  try { wx.showShareMenu && wx.showShareMenu({ withShareTicket: true, menus: ['shareAppMessage', 'shareTimeline'] }); } catch (e) {}
  try { wx.onShareAppMessage && wx.onShareAppMessage(function () { var s = getShare(); return { title: s.title, query: s.query }; }); } catch (e) {}
  try { wx.onShareTimeline && wx.onShareTimeline(function () { var s = getShare(); return { title: s.title, query: s.query }; }); } catch (e) {}

  return {
    // 复活按钮的字：广告坏了就写明免费
    reviveLabel: function () { return adState === 'error' ? '免费复活（广告暂不可用）' : '看广告复活'; },
    adState: function () { return adState; },
    // cb(how)：how = 'ad' 看完广告 / 'free' 广告不可用直接给 / null 中途关掉（不给复活）
    revive: function (cb) {
      function free() { adState = 'error'; stats.freeRevives++; cb('free'); }
      if (!ad || adState === 'error') return free();
      pending = cb; stats.adShows++;
      ad.show().catch(function () {
        // 第一次没拉到：再拉一次，还不行就直接复活
        return ad.load().then(function () { return ad.show(); });
      }).catch(function () { pending = null; free(); });
    },
    share: function () {
      var s = getShare();
      stats.shares++;
      try {
        wx.shareAppMessage({ title: s.title, query: s.query, fail: function () { stats.shareFails++; toast('当前环境不支持转发，可以截图分享'); } });
      } catch (e) { stats.shareFails++; }
    },
    toast: toast,
    stats: stats
  };
}
module.exports = { make: make };
