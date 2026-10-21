import 'package:flutter/services.dart';

import 'controller.dart';
import 'events.dart';

/// 把 Flutter 的软键盘 / 硬件键盘接到引擎里聚焦的那个 `<input>` / `<textarea>`。
///
/// 引擎只认「提交的文字」与功能键，所以这里把输入法每次给的完整值与上一次比
/// （按 Unicode 码点比，与引擎按 `char` 计数一致；按 UTF-16 比的话 emoji 会删成半个）：
///
/// 1. 取公共前缀 `p` 与公共后缀 `s`，中间那段 `[p, 旧长度 - s)` 是被换掉的；
/// 2. 先把引擎光标挪到被换掉那段的末尾（`left` / `right`），用 backspace 删掉，再送新文字；
/// 3. 最后按输入法给的选区把引擎的光标 / 选区摆到同一处。
///
/// 第 2 步以前假设「光标永远在末尾」：在文字中间点一下再打字、输入法自动纠错改前面的词、
/// 硬件方向键挪光标，引擎那边都会删错地方。引擎的光标位置由 `inputFocus.cursor` 给出，
/// 之后每一步都由这里记账（[_cursor] / [_anchor]）。
///
/// 拼音组字过程中的字母也会先进去、上屏时被替换 —— 引擎还没有组字接口。
class MiniTextInput implements TextInputClient {
  MiniTextInput(this.controller);

  final MiniProgramController controller;
  TextInputConnection? _conn;
  TextEditingValue _value = TextEditingValue.empty;
  String? _id;

  /// 引擎里的光标与选区锚点（码点下标）。没有选区时两者相等
  int _cursor = 0;
  int _anchor = 0;
  bool _keyHandler = false;

  bool get attached => _conn?.attached ?? false;

  /// 引擎发来 `inputFocus`：按输入框属性打开对应的键盘。
  ///
  /// 已经聚焦的输入框再点一下，引擎会再发一次 `inputFocus`（光标挪到点中的位置）：
  /// 这时不重开键盘，只把输入法那边的光标对齐过去。
  void open(MiniInputFocus f) {
    final runes = f.value.runes.length;
    final cursor = (f.cursor < 0 || f.cursor > runes) ? runes : f.cursor;
    final value = TextEditingValue(
      text: f.value,
      selection: TextSelection.collapsed(offset: _utf16Offset(f.value, cursor)),
    );
    _cursor = cursor;
    _anchor = cursor;
    if (_id == f.id && attached) {
      _value = value;
      _conn!.setEditingState(value);
      return;
    }
    close();
    _id = f.id;
    _value = value;
    final conn = TextInput.attach(
      this,
      TextInputConfiguration(
        inputType: _keyboard(f),
        obscureText: f.password || f.inputType == 'safe-password',
        autocorrect: false,
        enableSuggestions: false,
        inputAction: _action(f),
      ),
    );
    conn.setEditingState(_value);
    conn.show();
    _conn = conn;
    if (!_keyHandler) {
      HardwareKeyboard.instance.addHandler(_onHardwareKey);
      _keyHandler = true;
    }
  }

  /// 引擎发来 `inputBlur`（确认、点了别处）：收键盘
  void close() {
    _conn?.close();
    _conn = null;
    _id = null;
    if (_keyHandler) {
      HardwareKeyboard.instance.removeHandler(_onHardwareKey);
      _keyHandler = false;
    }
  }

  static TextInputType _keyboard(MiniInputFocus f) {
    if (f.multiline) return TextInputType.multiline;
    switch (f.inputType) {
      case 'number':
        return TextInputType.number;
      case 'digit':
        return const TextInputType.numberWithOptions(decimal: true);
      case 'idcard':
        return TextInputType.visiblePassword;
      default:
        return TextInputType.text;
    }
  }

  static TextInputAction _action(MiniInputFocus f) {
    // textarea 默认 `return`：右下角是换行键。写了 send / search … 时与 input 一样是确认键
    if (f.multiline && (f.confirmType.isEmpty || f.confirmType == 'return')) return TextInputAction.newline;
    switch (f.confirmType) {
      case 'send':
        return TextInputAction.send;
      case 'search':
        return TextInputAction.search;
      case 'next':
        return TextInputAction.next;
      case 'go':
        return TextInputAction.go;
      default:
        return TextInputAction.done;
    }
  }

  @override
  TextEditingValue? get currentTextEditingValue => _value;

  @override
  AutofillScope? get currentAutofillScope => null;

  @override
  void updateEditingValue(TextEditingValue value) {
    final old = _value.text.runes.toList();
    final now = value.text.runes.toList();
    var p = 0;
    while (p < old.length && p < now.length && old[p] == now[p]) {
      p++;
    }
    var s = 0;
    while (s < old.length - p && s < now.length - p && old[old.length - 1 - s] == now[now.length - 1 - s]) {
      s++;
    }
    final delEnd = old.length - s;
    final inserted = now.sublist(p, now.length - s);
    if (delEnd > p || inserted.isNotEmpty) {
      _moveTo(delEnd, old.length);
      for (var i = p; i < delEnd; i++) {
        controller.sendKey('backspace');
      }
      if (inserted.isNotEmpty) controller.sendText(String.fromCharCodes(inserted));
      _cursor = p + inserted.length;
      _anchor = _cursor;
    }
    // 只挪了光标 / 改了选区（硬件方向键、Gboard 空格键拖光标、输入法菜单里的全选）也要跟过去
    final sel = value.selection;
    if (sel.isValid) {
      _select(_runeIndex(value.text, sel.baseOffset), _runeIndex(value.text, sel.extentOffset), now.length);
    }
    _value = value;
  }

  /// 把引擎光标挪到 `target`（先收拢选区）
  void _moveTo(int target, int length) {
    if (_anchor != _cursor) {
      // 引擎里有选区时 → 只收拢到选区右端，不多走
      controller.sendKey('right');
      _cursor = _anchor > _cursor ? _anchor : _cursor;
      _anchor = _cursor;
    }
    if (target == _cursor) return;
    if (target == 0 || target == length) {
      controller.sendKey(target == 0 ? 'doc-start' : 'doc-end');
    } else {
      final key = target < _cursor ? 'left' : 'right';
      for (var i = (target - _cursor).abs(); i > 0; i--) {
        controller.sendKey(key);
      }
    }
    _cursor = target;
    _anchor = target;
  }

  /// 把引擎选区摆成 [base, extent)（相等就是一个光标）
  void _select(int base, int extent, int length) {
    if (base == _anchor && extent == _cursor) return;
    if (base == extent) {
      _moveTo(extent, length);
      return;
    }
    if (base == 0 && extent == length) {
      controller.sendKey('selectall');
      _anchor = 0;
      _cursor = length;
      return;
    }
    _moveTo(base, length);
    final key = extent < base ? 'shift+left' : 'shift+right';
    for (var i = (extent - base).abs(); i > 0; i--) {
      controller.sendKey(key);
    }
    _cursor = extent;
  }

  /// 输入法自己不处理、又是最常用的那几个硬件键：在这里改好值，再走同一条差分送给引擎，
  /// 并把新值回给输入法 —— 两边始终是同一个状态。
  ///
  /// 方向键、Home / End 交给平台（Android 的输入连接会改选区，再由 [updateEditingValue]
  /// 同步过去）；撤销不接（引擎撤销后的值这里无从得知，接了两边就会分叉）。
  bool _onHardwareKey(KeyEvent e) {
    if (!attached || e is KeyUpEvent) return false;
    final hk = HardwareKeyboard.instance;
    final primary = hk.isControlPressed || hk.isMetaPressed;
    final k = e.logicalKey;
    final v = _value;
    final sel = v.selection.isValid ? v.selection : TextSelection.collapsed(offset: v.text.length);
    if (k == LogicalKeyboardKey.backspace || k == LogicalKeyboardKey.delete) {
      if (sel.isCollapsed) {
        final at = _runeIndex(v.text, sel.extentOffset);
        final runes = v.text.runes.toList();
        final back = k == LogicalKeyboardKey.backspace;
        if (back ? at == 0 : at >= runes.length) return true;
        final from = back ? at - 1 : at;
        runes.removeAt(from);
        final text = String.fromCharCodes(runes);
        _applyLocal(TextEditingValue(text: text, selection: TextSelection.collapsed(offset: _utf16Offset(text, from))));
      } else {
        _applyLocal(_replaceSelection(v, sel, ''));
      }
      return true;
    }
    if (!primary) return false;
    if (k == LogicalKeyboardKey.keyA) {
      _applyLocal(v.copyWith(selection: TextSelection(baseOffset: 0, extentOffset: v.text.length)));
      return true;
    }
    if (k == LogicalKeyboardKey.keyC || k == LogicalKeyboardKey.keyX) {
      if (sel.isCollapsed) return true;
      Clipboard.setData(ClipboardData(text: sel.textInside(v.text)));
      if (k == LogicalKeyboardKey.keyX) _applyLocal(_replaceSelection(v, sel, ''));
      return true;
    }
    if (k == LogicalKeyboardKey.keyV) {
      Clipboard.getData(Clipboard.kTextPlain).then((data) {
        final text = data?.text;
        if (text == null || text.isEmpty || !attached) return;
        final cur = _value;
        final s = cur.selection.isValid ? cur.selection : TextSelection.collapsed(offset: cur.text.length);
        _applyLocal(_replaceSelection(cur, s, text));
      });
      return true;
    }
    return false;
  }

  /// 本地改了值：送给引擎（同一条差分），再告诉输入法
  void _applyLocal(TextEditingValue v) {
    updateEditingValue(v);
    _conn?.setEditingState(v);
  }

  static TextEditingValue _replaceSelection(TextEditingValue v, TextSelection sel, String text) {
    final start = sel.start, end = sel.end;
    final out = v.text.replaceRange(start, end, text);
    return TextEditingValue(text: out, selection: TextSelection.collapsed(offset: start + text.length));
  }

  /// UTF-16 下标 → 码点下标
  static int _runeIndex(String s, int utf16) {
    final end = utf16.clamp(0, s.length);
    return s.substring(0, end).runes.length;
  }

  /// 码点下标 → UTF-16 下标
  static int _utf16Offset(String s, int runeIndex) {
    var i = 0;
    var n = 0;
    for (final r in s.runes) {
      if (n == runeIndex) break;
      i += r > 0xFFFF ? 2 : 1;
      n++;
    }
    return i;
  }

  @override
  void performAction(TextInputAction action) {
    // 多行输入框的换行键：换行符已经由平台写进了编辑值（随后的 updateEditingValue 会把它送过去），
    // 这里再送一次就是两个换行 —— iOS 上一按回车出两个「□」就是这么来的。
    // Flutter 自己的 EditableText 对 newline 同样什么都不做。
    if (action == TextInputAction.newline) return;
    controller.sendKey('confirm');
  }

  @override
  void connectionClosed() {
    _conn = null;
    controller.sendKey('escape');
  }

  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}

  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {}

  @override
  void showAutocorrectionPromptRect(int start, int end) {}

  @override
  void insertTextPlaceholder(Size size) {}

  @override
  void removeTextPlaceholder() {}

  @override
  void showToolbar() {}

  @override
  void didChangeInputControl(TextInputControl? oldControl, TextInputControl? newControl) {}

  @override
  void performSelector(String selectorName) {}

  @override
  void insertContent(KeyboardInsertedContent content) {}
}
