// 入口页里的商店部分：搜索框、「最近使用」横排、分类、「全部小程序」四列宫格，各自一张白卡片。
// 点图标打开，长按弹操作单（收藏到「我的小程序」/ 从最近使用删除，store_sheets.dart）；
// 收藏过的图标右上角带一颗星。原生示例 StoreActivity.kt 是同一套。
import 'package:flutter/material.dart';

import 'mini_store.dart';
import 'store_links.dart';
import 'store_pages.dart' show StoreAppIcon;
import 'store_search.dart';
import 'store_sheets.dart';

const _green = Color(0xFF07C160);

/// 白底圆角卡片（入口页每一段都是一张）
class HomeCard extends StatelessWidget {
  const HomeCard({super.key, this.title, this.trailing, required this.child, this.padding = const EdgeInsets.only(bottom: 12)});

  final String? title;
  final Widget? trailing;
  final Widget child;
  final EdgeInsets padding;

  @override
  Widget build(BuildContext context) => Container(
        margin: const EdgeInsets.fromLTRB(12, 0, 12, 12),
        decoration: BoxDecoration(color: Colors.white, borderRadius: BorderRadius.circular(14)),
        clipBehavior: Clip.antiAlias,
        padding: padding,
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          if (title != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 4, 2),
              child: SizedBox(
                height: 36,
                child: Row(children: [
                  Expanded(child: Text(title!, style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600))),
                  ?trailing,
                ]),
              ),
            ),
          child,
        ]),
      );
}

class StoreSection extends StatefulWidget {
  const StoreSection({super.key});

  @override
  State<StoreSection> createState() => _StoreSectionState();
}

class _StoreSectionState extends State<StoreSection> {
  final _store = StoreLinks.store;
  StoreList? _list;
  List<StoreApp> _recent = const [];
  Set<String> _fav = const {};
  bool _loading = false;

  static const _catOrder = ['美食', '购物', '出行', '生活服务', '医疗健康', '金融', '工具', '教育', '娱乐', '资讯', '政务', '其他'];

  @override
  void initState() {
    super.initState();
    _refresh();
    // 在下拉面板 / 胶囊菜单 / 搜索页里收藏、删除：这里跟着变
    _store.localChanges.addListener(_reloadLocal);
  }

  @override
  void dispose() {
    _store.localChanges.removeListener(_reloadLocal);
    super.dispose();
  }

  Future<void> _refresh() async {
    setState(() => _loading = true);
    final l = await _store.list();
    if (!mounted) return;
    setState(() {
      _list = l;
      _loading = false;
    });
    await _reloadLocal();
  }

  Future<void> _reloadLocal() async {
    final r = await _store.recent();
    final f = await _store.favorites();
    if (mounted) {
      setState(() {
        _recent = r;
        _fav = {for (final a in f) a.id};
      });
    }
  }

  // 场景值 1001：发现栏小程序主入口（微信「小程序」列表点进去就是它）
  void _open(StoreApp app) => StoreLinks.open(StoreLink(key: app.key, scene: 1001), hint: app);

  void _search({String category = ''}) =>
      Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => StoreSearchPage(category: category)));

  Widget _cell(StoreApp app) => Semantics(
        button: true,
        label: '打开小程序 ${app.name}${_fav.contains(app.id) ? '（已收藏）' : ''}',
        hint: '长按更多操作',
        excludeSemantics: true,
        child: InkWell(
          borderRadius: BorderRadius.circular(10),
          onTap: () => _open(app),
          onLongPress: () => showAppActions(context, app),
          child: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
            Stack(clipBehavior: Clip.none, children: [
              StoreAppIcon.of(app, size: 50),
              if (_fav.contains(app.id))
                const Positioned(
                  right: -3,
                  top: -3,
                  child: DecoratedBox(
                    decoration: BoxDecoration(color: Colors.white, shape: BoxShape.circle),
                    child: Padding(padding: EdgeInsets.all(1.5), child: Icon(Icons.star_rounded, size: 15, color: Color(0xFFFFB400))),
                  ),
                ),
            ]),
            const SizedBox(height: 8),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              child: Text(app.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 12)),
            ),
          ]),
        ),
      );

  @override
  Widget build(BuildContext context) {
    final l = _list;
    final counts = <String, int>{};
    for (final a in l?.apps ?? const <StoreApp>[]) {
      final c = a.category.isEmpty ? '其他' : a.category;
      counts[c] = (counts[c] ?? 0) + 1;
    }
    final cellW = (MediaQuery.sizeOf(context).width - 24 - 16) / 4;
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      // 假搜索框：点了进搜索页（微信也是这样，入口页上不直接输入）
      Padding(
        padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
        child: Semantics(
          button: true,
          label: '搜索小程序',
          excludeSemantics: true,
          child: Material(
            color: Colors.white,
            borderRadius: BorderRadius.circular(10),
            child: InkWell(
              borderRadius: BorderRadius.circular(10),
              onTap: _search,
              child: const SizedBox(
                height: 40,
                child: Row(mainAxisAlignment: MainAxisAlignment.center, children: [
                  Icon(Icons.search, size: 18, color: Colors.black38),
                  SizedBox(width: 6),
                  Text('搜索小程序', style: TextStyle(fontSize: 15, color: Colors.black38)),
                ]),
              ),
            ),
          ),
        ),
      ),
      if (_recent.isNotEmpty)
        HomeCard(
          title: '最近使用',
          trailing: const Padding(
            padding: EdgeInsets.only(right: 12),
            child: Text('长按可删除', style: TextStyle(fontSize: 12, color: Colors.black38)),
          ),
          child: SizedBox(
            height: 92,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 8),
              children: [for (final a in _recent) SizedBox(width: cellW, child: _cell(a))],
            ),
          ),
        ),
      if (counts.isNotEmpty)
        HomeCard(
          title: '分类',
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 4, 16, 0),
            child: Wrap(spacing: 8, runSpacing: 8, children: [
              for (final c in _catOrder)
                if ((counts[c] ?? 0) > 0) StoreChip(label: '$c ${counts[c]}', onTap: () => _search(category: c)),
            ]),
          ),
        ),
      HomeCard(
        title: '全部小程序',
        trailing: _loading
            ? const Padding(
                padding: EdgeInsets.all(12),
                child: SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2)))
            : IconButton(tooltip: '刷新列表', icon: const Icon(Icons.refresh, size: 20), onPressed: _refresh),
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(children: [
              Icon(l?.offline == true ? Icons.cloud_off : Icons.cloud_done_outlined,
                  size: 14, color: l?.offline == true ? const Color(0xFFFA9D3B) : _green),
              const SizedBox(width: 4),
              Expanded(
                child: Text(
                  l == null
                      ? '正在连接 $kMiniStoreUrl …'
                      : l.offline
                          ? '服务器连不上，显示上次的列表（${l.error}）'
                          : '来自 $kMiniStoreUrl · ${l.apps.length} 个',
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 12, color: Colors.black45),
                ),
              ),
            ]),
          ),
          if (l != null && l.apps.isEmpty)
            const Padding(padding: EdgeInsets.all(16), child: Text('服务器上还没有小程序')),
          if (l != null && l.apps.isNotEmpty)
            GridView.count(
              crossAxisCount: 4,
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              padding: const EdgeInsets.fromLTRB(8, 12, 8, 0),
              childAspectRatio: 0.82,
              children: [for (final app in l.apps) _cell(app)],
            ),
        ]),
      ),
    ]);
  }
}
