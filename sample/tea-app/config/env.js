"use strict";
/**
 * MOCK = true：离线演示模式（默认）。所有接口由 api/mock.js 本地返回、图片走包内 /static，
 * 不访问后端，页面不会弹「网络异常」。接真实后端时改成 false，其余代码不用动。
 */
const MOCK = true;
const BASE_URL = "https://wx.51aihelp.com/api/front";
const ASSET_BASE = "https://wx.51aihelp.com/crmebimage/public/ui";
const FONT_BASE = "https://wx.51aihelp.com/fonts";
const FONT_VER = "2";
const TOKEN_HEADER = "Authori-zation";
const TOKEN_KEY = "yuncha_token";
const USER_KEY = "yuncha_user";
const TIMEOUT = 15e3;
exports.MOCK = MOCK;
exports.ASSET_BASE = ASSET_BASE;
exports.BASE_URL = BASE_URL;
exports.FONT_BASE = FONT_BASE;
exports.FONT_VER = FONT_VER;
exports.TIMEOUT = TIMEOUT;
exports.TOKEN_HEADER = TOKEN_HEADER;
exports.TOKEN_KEY = TOKEN_KEY;
exports.USER_KEY = USER_KEY;
//# sourceMappingURL=../../.sourcemap/mp-weixin/config/env.js.map
