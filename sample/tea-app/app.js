"use strict";
Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const common_vendor = require("./common/vendor.js");
const store_index = require("./store/index.js");
const utils_assets = require("./utils/assets.js");
if (!Math) {
  "./pages/splash/splash.js";
  "./pages/index/index.js";
  "./pages/choose/choose.js";
  "./pages/gift/gift.js";
  "./pages/origin/origin.js";
  "./pages/mine/mine.js";
  "./pages/ai-result/ai-result.js";
  "./pages/recommend/recommend.js";
  "./pages/category/category.js";
  "./pages/cart/cart.js";
  "./pages/adoption/list.js";
  "./pages/adoption/detail.js";
  "./pages/checkout/checkout.js";
  "./pages/checkout/success.js";
  "./pages/shop/list.js";
  "./pages/shop/detail.js";
  "./pages/cart/checkout.js";
  "./pages/cart/order-success.js";
  "./pages/orders/orders.js";
  "./pages/orders/detail.js";
  "./pages/my-garden/my-garden.js";
  "./pages/plate/plate.js";
  "./pages/trace/trace.js";
  "./pages/garden/garden.js";
  "./pages/process/process.js";
  "./pages/tea-king/tea-king.js";
  "./pages/brand/brand.js";
  "./pages/search/search.js";
  "./pages/recharge/recharge.js";
  "./pages/checkin/checkin.js";
  "./pages/coupons/coupons.js";
  "./pages/points/points.js";
  "./pages/address/address.js";
  "./pages/login/login.js";
  "./pages/subscribe/subscribe.js";
}
const _sfc_main = common_vendor.defineComponent({
  onLaunch: function() {
    store_index.initStore();
    utils_assets.refreshAssets();
    common_vendor.index.loadFontFace({
      global: true,
      family: "Playfair Display",
      source: 'url("https://wx.51aihelp.com/fonts/PlayfairDisplay-Regular.woff")',
      success: () => {
        console.log("Playfair Display loaded");
      },
      fail: (e) => {
        console.log("Playfair load fail", e);
      }
    });
    common_vendor.index.loadFontFace({
      global: true,
      family: "Source Han Serif SC",
      source: 'url("https://wx.51aihelp.com/fonts/SourceHanSerifSC-Heavy.woff")',
      success: () => {
        console.log("Source Han Serif SC Heavy loaded");
      },
      fail: (e) => {
        console.log("Source Han Serif load fail", e);
      }
    });
    console.log("凤凰云岫 App Launch");
  },
  onShow: function() {
    console.log("App Show");
  },
  onHide: function() {
    console.log("App Hide");
  }
});
function createApp() {
  const app = common_vendor.createSSRApp(_sfc_main);
  return {
    app
  };
}
createApp().app.mount("#app");
exports.createApp = createApp;
