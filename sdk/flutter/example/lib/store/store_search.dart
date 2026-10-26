// 搜索小程序（照微信「搜一搜 · 小程序」，与原生示例 SearchActivity.kt 同一套行为）：
// - 没输入：搜索历史（可清空）、热门搜索（前三名标色）、按分类浏览；
// - 输入中：联想词（停 200ms 再问服务器，过期的回包丢掉）；
// - 回车 / 点联想词 / 点历史：结果列表（名称里命中的字标绿），上面一排分类可以再筛；
// - 从入口页点某个分类进来（[StoreSearchPage.category]）：直接是这个分类的结果。
// 搜索走服务器的 `/api/apps?q=&category=`（tools/mini-store/store_search.py）。
import 'dart:async';

import 'package:flutter/material.dart';

import 'mini_store.dart';
import 'store_links.dart';
import 'store_pages.dart' show StoreAppIcon;
import 'store_sheets.dart' show showAppActions;

const _green = Color(0xFF07C160);

/// 灰底圆角的小标签（历史、分类）；[selected] 时绿字浅绿底
class StoreChip extends StatelessWidget {
  const StoreChip({super.key, required this.label, required this.onTap, this.selected = false});

  final String label;
  final VoidCallback onTap;
  final bool selected;

  @override
  Widget build(BuildContext context) => Material(
        color: selected ? const Color(0x1A07C160) : const Color(0xFFF7F7F7),
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            child: Text(label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 13, color: selected ? _green : Colors.black87)),
          ),
        ),
      );
}

enum _Mode { idle, suggest, results }

class StoreSearchPage extends StatefulWidget {
  const StoreSearchPage({super.key, this.category = ''});

  final String category;

  @override
  State<StoreSearchPage> createState() => _StoreSearchState();
}

class _StoreSearchState extends State<StoreSearchPage> {
  final _store = StoreLinks.store;
  final _input = TextEditingController();
  final _focus = FocusNode();
  late String _category = widget.category;
  _Mode _mode = _Mode.idle;
  List<String> _history = const [];
  List<String> _hot = const [];
  List<String> _suggest = const [];
  List<(String, int)> _cats = const [];
  List<StoreApp> _results = const [];
  int _total = 0;
  String _q = '';
  String? _status;
  Timer? _debounce;

  /// 每发一次请求 +1：回包对不上就是过期的（用户又打了字）
  int _seq = 0;

  @override
  void initState() {
    super.initState();
    _loadIdle();
    _store.categories().then((c) => mounted ? setState(() => _cats = c) : null).catchError((_) => null);
    if (_category.isNotEmpty) {
      _results0('');
    } else {
      WidgetsBinding.instance.addPostFrameCallback((_) => _focus.requestFocus());
    }
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _input.dispose();
    _focus.dispose();
    super.dispose();
  }

  Future<void> _loadIdle() async {
    final h = await _store.history();
    if (mounted) setState(() => _history = h);
    try {
      final hot = await _store.hotKeywords();
      if (mounted) setState(() => _hot = hot);
    } catch (_) {}
  }

  void _onChanged(String v) {
    _debounce?.cancel();
    final q = v.trim();
    if (q.isEmpty) {
      _seq++;
      setState(() => _mode = _category.isEmpty ? _Mode.idle : _Mode.results);
      if (_category.isNotEmpty) _results0('');
      return;
    }
    _debounce = Timer(const Duration(milliseconds: 200), () async {
      final my = ++_seq;
      final words = await _store.suggest(q).catchError((_) => <String>[]);
      if (!mounted || my != _seq) return;
      setState(() {
        _mode = _Mode.suggest;
        _q = q;
        // 第一行永远是输入的原文：联想不到也能直接搜
        _suggest = [q, ...words.where((w) => w != q)];
      });
    });
  }

  void _submit(String v) {
    final q = v.trim();
    if (q.isEmpty && _category.isEmpty) return;
    _debounce?.cancel();
    if (q.isNotEmpty) _store.addHistory(q).then((_) => _loadIdle());
    if (_input.text != q) {
      _input.value = TextEditingValue(text: q, selection: TextSelection.collapsed(offset: q.length));
    }
    _focus.unfocus();
    _results0(q);
  }

  Future<void> _results0(String q) async {
    final my = ++_seq;
    setState(() {
      _mode = _Mode.results;
      _q = q;
      _status = '搜索中…';
      _results = const [];
    });
    try {
      final (apps, total) = await _store.search(q, category: _category);
      if (!mounted || my != _seq) return;
      setState(() {
        _results = apps;
        _total = total;
        _status = total == 0
            ? (q.isNotEmpty ? '没有找到「$q」相关的小程序，换个词试试' : '这个分类下还没有小程序')
            : '共 $total 个小程序';
      });
    } catch (e) {
      if (mounted && my == _seq) setState(() => _status = '搜索失败：$e');
    }
  }

  void _open(StoreApp app) => StoreLinks.open(StoreLink(key: app.key, scene: 1005), hint: app);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.white,
      body: SafeArea(
        child: Column(children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 8, 4, 8),
            child: Row(children: [
              Expanded(
                child: Container(
                  height: 36,
                  decoration: BoxDecoration(color: const Color(0xFFEDEDED), borderRadius: BorderRadius.circular(8)),
                  child: TextField(
                    controller: _input,
                    focusNode: _focus,
                    textInputAction: TextInputAction.search,
                    onChanged: _onChanged,
                    onSubmitted: _submit,
                    style: const TextStyle(fontSize: 15),
                    decoration: const InputDecoration(
                      hintText: '搜索小程序',
                      border: InputBorder.none,
                      isDense: true,
                      prefixIcon: Icon(Icons.search, size: 18, color: Colors.black38),
                      prefixIconConstraints: BoxConstraints(minWidth: 34),
                      contentPadding: EdgeInsets.symmetric(vertical: 9),
                    ),
                  ),
                ),
              ),
              TextButton(
                onPressed: () => Navigator.of(context).maybePop(),
                child: const Text('取消', style: TextStyle(fontSize: 15, color: Color(0xFF576B95))),
              ),
            ]),
          ),
          Expanded(child: switch (_mode) {
            _Mode.idle => _idleView(),
            _Mode.suggest => _suggestView(),
            _Mode.results => _resultsView(),
          }),
        ]),
      ),
    );
  }

  Widget _header(String t, {String? action, VoidCallback? onAction}) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 18, 4, 6),
        child: Row(children: [
          Expanded(child: Text(t, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600))),
          if (action != null)
            TextButton(onPressed: onAction, child: Text(action, style: const TextStyle(fontSize: 13, color: Colors.black54))),
        ]),
      );

  Widget _idleView() {
    return ListView(children: [
      if (_history.isNotEmpty) ...[
        _header('搜索历史', action: '清空', onAction: () => _store.clearHistory().then((_) => _loadIdle())),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Wrap(spacing: 8, runSpacing: 8, children: [
            for (final w in _history) StoreChip(label: w, onTap: () => _submit(w)),
          ]),
        ),
      ],
      if (_hot.isNotEmpty) ...[
        _header('热门搜索'),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Wrap(children: [
            for (var i = 0; i < _hot.length; i++)
              FractionallySizedBox(
                widthFactor: 0.5,
                child: InkWell(
                  onTap: () => _submit(_hot[i]),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 9),
                    child: Row(children: [
                      SizedBox(
                        width: 22,
                        child: Text('${i + 1}',
                            style: TextStyle(
                                fontSize: 14,
                                fontWeight: FontWeight.w700,
                                color: const [Color(0xFFFA5151), Color(0xFFFA9D3B), Color(0xFFF7C543)]
                                        .elementAtOrNull(i) ??
                                    Colors.black38)),
                      ),
                      Expanded(child: Text(_hot[i], maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 14))),
                    ]),
                  ),
                ),
              ),
          ]),
        ),
      ],
      if (_cats.isNotEmpty) ...[
        _header('按分类浏览'),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Wrap(spacing: 8, runSpacing: 8, children: [
            for (final (c, n) in _cats)
              StoreChip(label: '$c $n', onTap: () {
                _category = c;
                _results0(_input.text.trim());
              }),
          ]),
        ),
      ],
    ]);
  }

  Widget _suggestView() => ListView(children: [
        for (final w in _suggest)
          ListTile(
            dense: true,
            leading: const Icon(Icons.search, size: 18, color: Colors.black38),
            title: _highlight(w, _q, const TextStyle(fontSize: 15, color: Colors.black)),
            onTap: () => _submit(w),
          ),
      ]);

  Widget _resultsView() {
    return ListView(children: [
      SizedBox(
        height: 44,
        child: ListView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          children: [
            for (final (c, _) in [('', 0), ..._cats])
              Padding(
                padding: const EdgeInsets.only(right: 8),
                child: StoreChip(
                  label: c.isEmpty ? '全部' : c,
                  selected: c == _category,
                  onTap: () {
                    _category = c;
                    _results0(_q);
                  },
                ),
              ),
          ],
        ),
      ),
      if (_status != null)
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 6, 16, 4),
          child: Text(_status!, style: const TextStyle(fontSize: 13, color: Colors.black54)),
        ),
      for (final a in _results)
        InkWell(
          onTap: () => _open(a),
          // 长按：收藏到「我的小程序」（与入口页、下拉面板同一张操作单）
          onLongPress: () => showAppActions(context, a, scene: 1005),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            child: Row(children: [
              StoreAppIcon.of(a, size: 48),
              const SizedBox(width: 12),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  _highlight(a.name, _q, const TextStyle(fontSize: 16, color: Colors.black)),
                  if (a.category.isNotEmpty || a.developer.isNotEmpty)
                    Text([a.category, a.developer].where((t) => t.isNotEmpty).join(' · '),
                        style: const TextStyle(fontSize: 12, color: Colors.black54)),
                  if (a.description.isNotEmpty)
                    Text(a.description,
                        maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 13, color: Colors.black54)),
                ]),
              ),
            ]),
          ),
        ),
      if (_total > _results.length)
        Padding(
          padding: const EdgeInsets.all(16),
          child: Text('只显示前 ${_results.length} 个，换个更具体的词', style: const TextStyle(fontSize: 12, color: Colors.black38)),
        ),
    ]);
  }

  /// 名称里出现的关键词标绿（每段关键词各标一次）
  static Widget _highlight(String text, String q, TextStyle style) {
    final marks = List.filled(text.length, false);
    for (final t in q.split(' ').where((t) => t.isNotEmpty)) {
      final i = text.toLowerCase().indexOf(t.toLowerCase());
      if (i >= 0) {
        for (var k = i; k < i + t.length && k < text.length; k++) {
          marks[k] = true;
        }
      }
    }
    final spans = <TextSpan>[];
    for (var i = 0; i < text.length;) {
      var j = i;
      while (j < text.length && marks[j] == marks[i]) {
        j++;
      }
      spans.add(TextSpan(text: text.substring(i, j), style: marks[i] ? style.copyWith(color: _green) : style));
      i = j;
    }
    return Text.rich(TextSpan(children: spans), maxLines: 1, overflow: TextOverflow.ellipsis);
  }
}
