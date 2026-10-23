import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:mini_render/mini_render.dart';

/// 例子的 hostApis：引擎做不了、插件也没自带，要 App 自己接的那些（登录、支付、定位、扫码……）。
///
/// 真实 App 在这里接自己的账号体系、支付 SDK、地图 / 定位 SDK、扫码库。例子给演示数据；要用户点头的
/// （支付、拨号、授权、选位置、选地址……）弹 Flutter 自己的界面 —— 表明调用确实走到了宿主、结果是宿主给的。
/// 用户取消一律 `<api>:fail cancel`（与微信一致，页面的 fail 分支接得住）。
///
/// 选图 / 拍照 / 选视频 / 选文件 / 存相册 / 打开文档**不在这里**：插件自带（[MiniBuiltinHostApis]），
/// 走系统自己的选择器。小程序侧的演示见内置示例的「宿主」tab（`assets/demo/pages/host`）。
class ExampleHostApis {
  ExampleHostApis(this._context);

  /// 弹界面用的 context（页面已经关了就是 null，这时一律当作取消）
  final BuildContext? Function() _context;

  /// 微信里「要宿主才能做」的那一组（system.js 的默认 fail 名单）去掉插件自带的，全都接上
  Map<String, MiniHostApi> get all => {
        // ── 账号与授权 ──
        'login': (a) async => {'code': 'demo_${DateTime.now().millisecondsSinceEpoch.toRadixString(36)}'},
        'checkSession': (a) async => {},
        'getUserProfile': (a) async {
          if (!await _confirm('授权个人信息', '小程序申请获取你的昵称、头像\n用途：${a['desc'] ?? ''}', '允许')) {
            throw const MiniHostApiError('getUserProfile:fail auth deny');
          }
          return {'userInfo': _user};
        },
        'getUserInfo': (a) async => {
              'userInfo': _user,
              'rawData': jsonEncode(_user),
              'signature': 'demo_signature',
              'encryptedData': 'ZGVtbw==',
              'iv': 'ZGVtbw==',
            },
        'authorize': (a) async {
          final scope = '${a['scope'] ?? ''}';
          if (!await _confirm('授权', '小程序申请使用：${_scopeName(scope)}（$scope）', '允许')) {
            throw const MiniHostApiError('authorize:fail auth deny');
          }
          _granted.add(scope);
          return {};
        },
        'openSetting': (a) async => {
              'authSetting': {for (final s in _scopes.keys) s: _granted.contains(s)},
            },
        // ── 位置与地址 ──
        'getLocation': (a) async => {
              'latitude': 22.5431,
              'longitude': 114.0579,
              'speed': 0,
              'accuracy': 30,
              'altitude': 0,
              'verticalAccuracy': 0,
              'horizontalAccuracy': 30,
              'type': '${a['type'] ?? 'wgs84'}',
            },
        'chooseLocation': _chooseLocation,
        'openLocation': (a) async {
          await _confirm('查看位置', '${a['name'] ?? '位置'}\n${a['address'] ?? ''}\n${a['latitude']}, ${a['longitude']}（缩放 ${a['scale'] ?? 18}）', '好',
              cancel: false);
          return {};
        },
        'chooseAddress': (a) async {
          if (!await _confirm('选择收货地址', '张三 13800000000\n广东省深圳市南山区科技园 1 号', '使用这个地址')) {
            throw const MiniHostApiError('chooseAddress:fail cancel');
          }
          return {
            'userName': '张三',
            'postalCode': '518057',
            'provinceName': '广东省',
            'cityName': '深圳市',
            'countyName': '南山区',
            'streetName': '粤海街道',
            'detailInfo': '科技园 1 号',
            'detailInfoNew': '科技园 1 号',
            'nationalCode': '440305',
            'telNumber': '13800000000',
          };
        },
        // ── 支付与消息 ──
        'requestPayment': (a) async {
          final yuan = ((a['total'] as num? ?? 0) / 100).toStringAsFixed(2);
          if (!await _confirm('模拟支付', '小程序请求支付 ¥$yuan', '支付')) {
            throw const MiniHostApiError('requestPayment:fail cancel');
          }
          return {};
        },
        'requestSubscribeMessage': (a) async {
          final ids = [for (final t in (a['tmplIds'] as List? ?? const [])) '$t'];
          final ok = await _confirm('订阅消息', '小程序想给你发送 ${ids.length} 类通知', '允许');
          return {for (final t in ids) t: ok ? 'accept' : 'reject'};
        },
        'chooseInvoice': (a) async {
          if (!await _confirm('选择发票抬头', '示例科技有限公司\n税号 91440300000000000X', '使用')) {
            throw const MiniHostApiError('chooseInvoice:fail cancel');
          }
          return {
            'invoiceInfo': jsonEncode({'title': '示例科技有限公司', 'taxNumber': '91440300000000000X', 'type': 0}),
          };
        },
        'openCustomerServiceChat': (a) async {
          await _confirm('客服会话', '（演示）宿主在这里打开企业微信客服：${(a['extInfo'] as Map?)?['url'] ?? ''}', '好', cancel: false);
          return {};
        },
        // ── 设备 ──
        'scanCode': (a) async => {'result': 'https://example.com/qr/demo', 'scanType': 'QR_CODE', 'charSet': 'UTF-8', 'path': ''},
        'makePhoneCall': (a) async {
          if (!await _confirm('拨打电话', '${a['phoneNumber'] ?? ''}', '呼叫')) {
            throw const MiniHostApiError('makePhoneCall:fail cancel');
          }
          return {};
        },
        'chooseContact': (a) async => {
              'phoneNumber': '13800000000',
              'displayName': '李四',
              'phoneNumberList': ['13800000000', '010-12345678'],
            },
        'addPhoneContact': (a) async {
          final name = [a['lastName'], a['firstName']].whereType<Object>().join();
          if (!await _confirm('添加到通讯录', '$name ${a['mobilePhoneNumber'] ?? ''}', '添加')) {
            throw const MiniHostApiError('addPhoneContact:fail cancel');
          }
          return {};
        },
        'startSoterAuthentication': (a) async {
          if (!await _confirm('生物认证', '${a['authContent'] ?? '请验证'}（演示：点「验证」当作通过）', '验证')) {
            throw const MiniHostApiError('startSoterAuthentication:fail cancel', {'errCode': 90008});
          }
          final mode = '${(a['requestAuthModes'] as List?)?.first ?? 'fingerPrint'}';
          return {
            'authMode': mode,
            'resultJSON': jsonEncode({'raw': a['challenge'] ?? '', 'fid': 'demo'}),
            'resultJSONSignature': 'demo_signature',
            'errCode': 0,
          };
        },
        'getWeRunData': (a) async => {'encryptedData': 'ZGVtbw==', 'iv': 'ZGVtbw==', 'cloudID': 'demo_cloud_id'},
        // ── 跳转与分享 ──
        'navigateToMiniProgram': (a) async {
          if (!await _confirm('打开小程序', '即将打开 ${a['appId'] ?? ''}\n路径 ${a['path'] ?? '首页'}', '允许')) {
            throw const MiniHostApiError('navigateToMiniProgram:fail cancel');
          }
          return {};
        },
        // 只有被别的小程序打开时才回得去（与微信一致）；例子里小程序是从 Flutter 列表打开的
        'navigateBackMiniProgram': (a) async =>
            throw const MiniHostApiError('navigateBackMiniProgram:fail 当前小程序不是由其它小程序打开的'),
        'getShareInfo': (a) async => {'encryptedData': 'ZGVtbw==', 'iv': 'ZGVtbw==', 'cloudID': 'demo_cloud_id'},
      };

  static const _user = {'nickName': 'Flutter 用户', 'avatarUrl': '', 'gender': 0, 'language': 'zh_CN', 'country': '', 'province': '', 'city': ''};

  static const _scopes = {
    'scope.userLocation': '精确地理位置',
    'scope.userFuzzyLocation': '模糊地理位置',
    'scope.record': '麦克风',
    'scope.camera': '摄像头',
    'scope.writePhotosAlbum': '保存到相册',
    'scope.address': '通讯地址',
    'scope.invoiceTitle': '发票抬头',
    'scope.werun': '微信运动步数',
  };

  final Set<String> _granted = {'scope.writePhotosAlbum', 'scope.camera'};

  static String _scopeName(String scope) => _scopes[scope] ?? scope;

  /// 选位置：宿主的界面列几个附近的地点（真实 App 用地图 SDK 的选点页）
  Future<Map<String, dynamic>> _chooseLocation(Map<String, dynamic> a) async {
    final ctx = _context();
    if (ctx == null) throw const MiniHostApiError('chooseLocation:fail cancel');
    final lat = (a['latitude'] as num?)?.toDouble() ?? 39.908823;
    final lng = (a['longitude'] as num?)?.toDouble() ?? 116.39747;
    final pois = [
      ('天安门广场', '北京市东城区东长安街', 0.0, 0.0),
      ('故宫博物院', '北京市东城区景山前街 4 号', 0.0094, 0.0),
      ('国家博物馆', '北京市东城区东长安街 16 号', -0.0012, 0.0043),
      ('人民大会堂', '北京市西城区西长安街', -0.0012, -0.0044),
    ];
    final picked = await showModalBottomSheet<int>(
      context: ctx,
      builder: (c) => SafeArea(
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          const ListTile(title: Text('选择位置（宿主界面，演示数据）')),
          for (final (i, p) in pois.indexed)
            ListTile(
              leading: const Icon(Icons.place_outlined),
              title: Text(p.$1),
              subtitle: Text(p.$2),
              onTap: () => Navigator.pop(c, i),
            ),
        ]),
      ),
    );
    if (picked == null) throw const MiniHostApiError('chooseLocation:fail cancel');
    final p = pois[picked];
    return {'name': p.$1, 'address': p.$2, 'latitude': lat + p.$3, 'longitude': lng + p.$4};
  }

  /// 宿主自己的对话框。`cancel: false` 只有一个按钮（展示类的，点了就算 ok）
  Future<bool> _confirm(String title, String content, String okText, {bool cancel = true}) async {
    final ctx = _context();
    if (ctx == null) return false;
    final r = await showDialog<bool>(
      context: ctx,
      builder: (c) => AlertDialog(
        title: Text(title),
        content: Text(content),
        actions: [
          if (cancel) TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('取消')),
          FilledButton(onPressed: () => Navigator.pop(c, true), child: Text(okText)),
        ],
      ),
    );
    return r ?? !cancel;
  }
}
