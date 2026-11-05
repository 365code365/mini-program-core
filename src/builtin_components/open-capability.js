// <__KIND__>：要微信后台 / 宿主才有的开放能力组件（广告、公众号、小店、功能页）。引擎拿不到这些数据，
// 按官方「加载失败」的那条路走，让页面自己的兼容分支接住（官方 ad.html 的 tip：监听到 error 后隐藏父容器）：
// - ad / ad-custom：挂上后发 bind:error {errCode: 1004}（官方错误码表：无适合的广告），不占高度（官方：无广告时不占高度）；
// - official-account / official-account-publish：挂上后发 bind:error {status: 5, errMsg}（官方 status 表：5 场景值错误 ——
//   引擎不是从扫码场景打开的；errMsg 的原文官方没给，待验），不显示；
// - store-product / store-coupon / store-gift：显示使用方的 custom-content（slot），点了发 bind:entererror
//   {code: -1, message}（官方 detail = {code, message}，code 的取值官方没列，待验）；store-home 空白；
// - functional-page-navigator：显示 slot，点了发 bind:fail {errMsg}（官方：detail 格式与具体功能页相关）。
Component({
  data: {},
  lifetimes: {
    attached: function () {
      var k = '__KIND__', self = this, detail = null;
      if (k === 'ad' || k === 'ad-custom') detail = { errCode: 1004 };
      else if (k === 'official-account' || k === 'official-account-publish') detail = { status: 5, errMsg: k + ':fail scene not supported' };
      if (detail) setTimeout(function () { self.triggerEvent('error', detail); }, 0);
    }
  },
  methods: {
    _tap: function () {
      var k = '__KIND__';
      if (k === 'store-product' || k === 'store-coupon' || k === 'store-gift') {
        this.triggerEvent('entererror', { code: -1, message: '当前环境不支持跳转微信小店' });
      } else if (k === 'functional-page-navigator') {
        this.triggerEvent('fail', { errMsg: 'navigateToFunctionalPage:fail 当前环境不支持功能页' });
      }
    }
  }
});
