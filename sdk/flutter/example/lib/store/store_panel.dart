// 下拉出来的深色小程序面板（微信聊天页往下拉的那一屏）：搜索框、「最近使用」、「我的小程序」。
// 点图标打开，长按弹操作单（收藏 / 移出、从最近使用删除）。数据都是本机记的（[MiniStore.localChanges] 一变就刷新）。
// 原生示例 StorePanel.kt 是同一套布局与场景值。
import 'package:flutter/material.dart';

import 'mini_store.dart';
import 'store_drawer.dart';
import 'store_links.dart';
import 'store_pages.dart' show StoreAppIcon;
import 'store_search.dart';
import 'store_sheets.dart';

/// 微信的场景值：1089 下拉「最近使用」、1104 下拉「我的小程序」
const _sceneRecent = 1089;
const _sceneFavorite = 1104;

class MiniProgramPanel extends StatefulWidget {
  const MiniProgramPanel({super.key});

  @override
  State<MiniProgramPanel> createState() => _PanelState();
}

class _PanelState extends State<MiniProgramPanel> {
  final _store = StoreLinks.store;
  List<StoreApp> _recent = const [];
  List<StoreApp> _fav = const [];

  @override
  void initState() {
    super.initState();
    _load();
    _store.localChanges.addListener(_load);
  }

  @override
  void dispose() {
    _store.localChanges.removeListener(_load);
    super.dispose();
  }

  Future<void> _load() async {
    final r = await _store.recent();
    final f = await _store.favorites();
    if (mounted) {
      setState(() {
        _recent = r;
        _fav = f;
      });
    }
  }

  void _search() =>
      Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const StoreSearchPage()));

  Widget _header(String title, {String hint = ''}) => Padding(
        padding: const EdgeInsets.fromLTRB(20, 22, 20, 10),
        child: Row(children: [
          Text(title, style: const TextStyle(fontSize: 14, color: Colors.white70, fontWeight: FontWeight.w600)),
          const Spacer(),
          if (hint.isNotEmpty) Text(hint, style: const TextStyle(fontSize: 12, color: Colors.white30)),
        ]),
      );

  Widget _grid(List<StoreApp> apps, int scene) => LayoutBuilder(builder: (context, box) {
        final w = (box.maxWidth - 16) / 4;
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: Wrap(children: [
            for (final a in apps)
              SizedBox(width: w, child: _PanelCell(app: a, scene: scene)),
          ]),
        );
      });

  Widget _empty(String text) => Container(
        margin: const EdgeInsets.symmetric(horizontal: 16),
        padding: const EdgeInsets.symmetric(vertical: 22, horizontal: 16),
        decoration: BoxDecoration(
          color: Colors.white.withValues(alpha: 0.05),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: Colors.white12),
        ),
        child: Text(text, textAlign: TextAlign.center, style: const TextStyle(fontSize: 13, color: Colors.white38, height: 1.5)),
      );

  @override
  Widget build(BuildContext context) {
    final peek = PullDownDrawer.peekHeight(context);
    return DecoratedBox(
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Color(0xFF3B3B4F), Color(0xFF262633), Color(0xFF1C1C25)],
        ),
      ),
      child: SafeArea(
        bottom: false,
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          const SizedBox(
            height: 48,
            child: Center(
              child: Text('小程序', style: TextStyle(fontSize: 17, color: Colors.white, fontWeight: FontWeight.w600)),
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Semantics(
              button: true,
              label: '搜索小程序',
              excludeSemantics: true,
              child: Material(
                color: Colors.white.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(10),
                child: InkWell(
                  borderRadius: BorderRadius.circular(10),
                  onTap: _search,
                  child: const SizedBox(
                    height: 38,
                    child: Row(mainAxisAlignment: MainAxisAlignment.center, children: [
                      Icon(Icons.search, size: 18, color: Colors.white54),
                      SizedBox(width: 6),
                      Text('搜索小程序', style: TextStyle(fontSize: 15, color: Colors.white54)),
                    ]),
                  ),
                ),
              ),
            ),
          ),
          Expanded(
            child: ListView(
              // 内容不满一屏也要能拖：拉到底再往上推就是收起面板（store_drawer.dart 听越界）
              physics: const AlwaysScrollableScrollPhysics(parent: ClampingScrollPhysics()),
              padding: EdgeInsets.only(bottom: peek + 24),
              children: [
                _header('最近使用', hint: _recent.isEmpty ? '' : '长按可删除'),
                if (_recent.isEmpty) _empty('打开过的小程序会出现在这里') else _grid(_recent, _sceneRecent),
                _header('我的小程序', hint: _fav.isEmpty ? '' : '${_fav.length} 个'),
                if (_fav.isEmpty)
                  _empty('长按任意小程序，选「添加到我的小程序」\n或在小程序里点右上角 ••• 收藏')
                else
                  _grid(_fav, _sceneFavorite),
                const Padding(
                  padding: EdgeInsets.only(top: 28),
                  child: Text('上推或点下方返回', textAlign: TextAlign.center, style: TextStyle(fontSize: 12, color: Colors.white24)),
                ),
              ],
            ),
          ),
        ]),
      ),
    );
  }
}

class _PanelCell extends StatelessWidget {
  const _PanelCell({required this.app, required this.scene});

  final StoreApp app;
  final int scene;

  @override
  Widget build(BuildContext context) => Semantics(
        button: true,
        label: '打开${app.isGame ? '小游戏' : '小程序'} ${app.name}',
        hint: '长按更多操作',
        excludeSemantics: true,
        child: InkWell(
          borderRadius: BorderRadius.circular(10),
          onTap: () => StoreLinks.open(StoreLink(key: app.key, scene: scene), hint: app),
          onLongPress: () => showAppActions(context, app, scene: scene),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 10),
            child: Column(children: [
              StoreAppIcon.of(app, size: 50),
              const SizedBox(height: 8),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: Text(app.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 12, color: Colors.white70)),
              ),
            ]),
          ),
        ),
      );
}
