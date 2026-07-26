"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function preOrderByCart(cartIds) {
  const details = [];
  for (let i = 0; i < cartIds.length; i++) {
    details.push(new common_vendor.UTSJSONObject({ shoppingCartId: cartIds[i] }));
  }
  return utils_request.post("/order/pre/order", new common_vendor.UTSJSONObject({ preOrderType: "shoppingCart", orderDetails: details }));
}
function loadPre(preOrderNo) {
  return utils_request.get("/order/load/pre/" + preOrderNo, null);
}
function computedPrice(preOrderNo, addressId, couponId, useIntegral) {
  return utils_request.post("/order/computed/price", new common_vendor.UTSJSONObject({
    preOrderNo,
    addressId,
    couponId,
    useIntegral,
    shippingType: 1
  }));
}
function createOrder(preOrderNo, addressId, couponId, useIntegral, payType, payChannel, mark) {
  return utils_request.post("/order/create", new common_vendor.UTSJSONObject({
    preOrderNo,
    addressId,
    couponId,
    useIntegral,
    payType,
    payChannel,
    mark,
    shippingType: 1
  }));
}
function payment(orderNo, payType, payChannel) {
  return utils_request.post("/pay/payment", new common_vendor.UTSJSONObject({ uni: orderNo, orderNo, payType, payChannel }));
}
function orderList(type, page, limit) {
  return utils_request.get("/order/list", new common_vendor.UTSJSONObject({ type, page, limit }));
}
function orderDetail(orderId) {
  return utils_request.get("/order/detail/" + orderId, null);
}
function orderTake(id) {
  return utils_request.post("/order/take?id=" + id, null);
}
function orderCancel(id) {
  return utils_request.post("/order/cancel?id=" + id, null);
}
function orderCoupons(preOrderNo) {
  return utils_request.get("/coupons/order/" + preOrderNo, null);
}
function refundReason() {
  return utils_request.get("/order/refund/reason", null);
}
function orderRefund(id, text, explain, uni) {
  return utils_request.post("/order/refund", new common_vendor.UTSJSONObject({ id, text, explain, uni }));
}
function orderExpress(orderId) {
  return utils_request.get("/order/express/" + orderId, null);
}
exports.computedPrice = computedPrice;
exports.createOrder = createOrder;
exports.loadPre = loadPre;
exports.orderCancel = orderCancel;
exports.orderCoupons = orderCoupons;
exports.orderDetail = orderDetail;
exports.orderExpress = orderExpress;
exports.orderList = orderList;
exports.orderRefund = orderRefund;
exports.orderTake = orderTake;
exports.payment = payment;
exports.preOrderByCart = preOrderByCart;
exports.refundReason = refundReason;
