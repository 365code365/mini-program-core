/// 在 Flutter 里打开微信小程序（mini-render 引擎）。
///
/// ```dart
/// final c = MiniProgramController(appDir: dir);
/// Navigator.push(context, MaterialPageRoute(builder: (_) => MiniProgramScaffold(controller: c)));
/// // 关掉时 c.dispose()
/// ```
library;

export 'src/capsule.dart' show MiniCapsule, CapsulePainter;
export 'src/controller.dart' show MiniProgramController;
export 'src/events.dart';
export 'src/scaffold.dart' show MiniProgramScaffold;
export 'src/text_input.dart' show MiniTextInput;
export 'src/view.dart' show MiniProgramView, MiniNativeViewBuilder;
