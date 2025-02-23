import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'rust_binding.dart';
import 'login_page.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();

  // 初始化 RustBinding (MethodChannel)
  RustBinding().init();

  runApp(const ProviderScope(child: MyApp()));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    // 使用 Material3 + 主题色
    final theme = ThemeData(
      useMaterial3: true,
      colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
    );

    return MaterialApp(
      title: "Alphaflow Demo",
      theme: theme,
      // 初始页面 => LoginPage
      home: const LoginPage(),
    );
  }
}
