// 两个底部弹层，照微信的样子（原生示例 StoreSheets.kt 是同一套）：
// - 长按一个小程序：添加到 / 移出「我的小程序」、从「最近使用」删除、打开；
// - 小程序里胶囊的「•••」：顶上是小程序名（点了看关于），下面两排圆角方块（转发、收藏 / 重新进入、诊断、关于），
//   最下面隔开一条「取消」。
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../native_sheet.dart';
import 'mini_store.dart';
import 'store_links.dart';
import 'store_pages.dart' show StoreAppIcon;

const _sheetBg = Color(0xFFF7F7F7);
const _danger = Color(0xFFFA5151);

/// 白底圆角、底下让出手势条的那一层
Widget _frame(BuildContext ctx, List<Widget> children, {Color color = Colors.white}) => Container(
      decoration: BoxDecoration(color: color, borderRadius: const BorderRadius.vertical(top: Radius.circular(12))),
      clipBehavior: Clip.antiAlias,
      padding: EdgeInsets.only(bottom: MediaQuery.paddingOf(ctx).bottom),
      child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
    );

Widget _option(String label, VoidCallback onTap, {Color color = Colors.black87, FontWeight? weight}) => Material(
      color: Colors.white,
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          height: 56,
          child: Center(child: Text(label, style: TextStyle(fontSize: 17, color: color, fontWeight: weight))),
        ),
      ),
    );

void _toast(ScaffoldMessengerState? m, String text) {
  m
    ?..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(
      content: Text(text),
      behavior: SnackBarBehavior.floating,
      duration: const Duration(milliseconds: 1600),
    ));
}

/// 长按小程序的操作单：收藏 / 取消收藏、从最近使用删除、打开。改动经 [MiniStore.localChanges] 通知各处刷新
Future<void> showAppActions(BuildContext context, StoreApp app, {int scene = 1001}) async {
  HapticFeedback.mediumImpact();
  final store = StoreLinks.store;
  final fav = await store.isFavorite(app.id);
  final inRecent = (await store.recent()).any((a) => a.id == app.id);
  if (!context.mounted) return;
  final messenger = ScaffoldMessenger.maybeOf(context);
  await showModalBottomSheet<void>(
    context: context,
    backgroundColor: Colors.transparent,
    builder: (ctx) {
      void done(Future<void> Function() f) {
        Navigator.pop(ctx);
        f();
      }

      return _frame(ctx, color: _sheetBg, [
        Container(
          color: Colors.white,
          padding: const EdgeInsets.fromLTRB(20, 18, 20, 16),
          child: Row(children: [
            StoreAppIcon.of(app, size: 44),
            const SizedBox(width: 12),
            Expanded(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text(app.name, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                if (app.description.isNotEmpty || app.developer.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 3),
                    child: Text(app.description.isNotEmpty ? app.description : app.developer,
                        maxLines: 1, overflow: TextOverflow.ellipsis,
                        style: const TextStyle(fontSize: 12, color: Colors.black45)),
                  ),
              ]),
            ),
          ]),
        ),
        const Divider(height: 0.5, thickness: 0.5),
        _option(fav ? '从「我的小程序」移除' : '添加到「我的小程序」', () => done(() async {
              final now = await store.toggleFavorite(app);
              _toast(messenger, now ? '已添加到「我的小程序」，下拉首页可见' : '已从「我的小程序」移除');
            })),
        if (inRecent) ...[
          const Divider(height: 0.5, thickness: 0.5),
          _option('从「最近使用」删除', color: _danger, () => done(() async {
                await store.removeRecent(app.id);
                _toast(messenger, '已从「最近使用」删除');
              })),
        ],
        const Divider(height: 0.5, thickness: 0.5),
        _option('打开', () => done(() async => StoreLinks.open(StoreLink(key: app.key, scene: scene), hint: app))),
        const SizedBox(height: 8),
        _option('取消', () => Navigator.pop(ctx)),
      ]);
    },
  );
}

/// 胶囊菜单里的一格
class CapsuleMenuItem {
  const CapsuleMenuItem(this.icon, this.label, this.onTap,
      {this.enabled = true, this.active = false, this.id = '', this.glyph = '•'});

  final IconData icon;

  /// 原生弹层（lib/native_sheet.dart）回传用的键，与它画的字符图标
  final String id;
  final String glyph;
  final String label;
  final VoidCallback onTap;

  /// 灰掉不能点（例如内置示例没有商店链接，转发 / 收藏都做不了）
  final bool enabled;

  /// 图标标成主色（已收藏）
  final bool active;
}

/// 胶囊「•••」的底部菜单。[onHeader]：点顶上那行（小程序名）—— 示例里弹「关于」
Future<void> showCapsuleMenu(
  BuildContext context, {
  required String id,
  required String name,
  String iconUrl = '',
  String subtitle = '',
  VoidCallback? onHeader,
  required List<CapsuleMenuItem> top,
  required List<CapsuleMenuItem> bottom,
}) =>
    showModalBottomSheet<void>(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (ctx) {
        Widget row(List<CapsuleMenuItem> items) => SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.fromLTRB(12, 16, 12, 14),
              child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
                for (final it in items) _MenuCell(it, close: () => Navigator.pop(ctx)),
              ]),
            );
        return _frame(ctx, color: _sheetBg, [
          Material(
            color: _sheetBg,
            child: InkWell(
              onTap: onHeader == null
                  ? null
                  : () {
                      Navigator.pop(ctx);
                      onHeader();
                    },
              child: Padding(
                padding: const EdgeInsets.fromLTRB(20, 16, 12, 14),
                child: Row(children: [
                  StoreAppIcon(id: id, name: name, size: 30, url: iconUrl),
                  const SizedBox(width: 10),
                  // 名字 + 箭头占满剩下的宽（不用 Flexible + Spacer：两者平分剩余宽度，版本号会停在中间）
                  Expanded(
                    child: Row(children: [
                      Flexible(
                        child: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600)),
                      ),
                      if (onHeader != null) const Icon(Icons.chevron_right, size: 20, color: Colors.black38),
                    ]),
                  ),
                  if (subtitle.isNotEmpty) Text(subtitle, style: const TextStyle(fontSize: 12, color: Colors.black38)),
                ]),
              ),
            ),
          ),
          const Divider(height: 0.5, thickness: 0.5, indent: 20, endIndent: 20),
          row(top),
          const Divider(height: 0.5, thickness: 0.5, indent: 20, endIndent: 20),
          row(bottom),
          const SizedBox(height: 8),
          _option('取消', () => Navigator.pop(ctx)),
        ]);
      },
    );

class _MenuCell extends StatelessWidget {
  const _MenuCell(this.item, {required this.close});

  final CapsuleMenuItem item;
  final VoidCallback close;

  @override
  Widget build(BuildContext context) {
    final fg = !item.enabled
        ? Colors.black26
        : item.active
            ? const Color(0xFFFFB400)
            : Colors.black87;
    return Semantics(
      button: true,
      enabled: item.enabled,
      label: item.label,
      excludeSemantics: true,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: item.enabled
            ? () {
                close();
                item.onTap();
              }
            : null,
        child: SizedBox(
          width: 76,
          child: Column(children: [
            Container(
              width: 52,
              height: 52,
              decoration: BoxDecoration(color: Colors.white, borderRadius: BorderRadius.circular(12)),
              child: Icon(item.icon, color: fg, size: 26),
            ),
            const SizedBox(height: 8),
            Text(item.label,
                textAlign: TextAlign.center,
                maxLines: 2,
                style: TextStyle(fontSize: 11, height: 1.3, color: item.enabled ? Colors.black54 : Colors.black26)),
          ]),
        ),
      ),
    );
  }
}

/// 转发：微信是发给好友；示例没有好友链，复制一条能打开它的链接（外部 App / 浏览器里点开即进小程序）
void copyMiniLink(BuildContext context, StoreApp app, {String? route}) {
  final q = {'appid': app.appId.isNotEmpty ? app.appId : app.id, if (route != null && route.isNotEmpty) 'path': route};
  Clipboard.setData(ClipboardData(text: 'minirender://open?${Uri(queryParameters: q).query}'));
  final msg = '链接已复制：在别的 App 或浏览器里打开即可进入「${app.name}」';
  // 在小程序页上调：Android 用系统 Toast（盖在平台视图上的 Flutter 层每帧都贵，见 native_sheet.dart）
  final messenger = ScaffoldMessenger.maybeOf(context);
  if (NativeSheet.available) {
    NativeSheet.toast(msg).catchError((_) => _toast(messenger, msg));
    return;
  }
  _toast(messenger, msg);
}
