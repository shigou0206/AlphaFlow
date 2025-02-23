import 'dart:convert';
import 'package:flutter/material.dart';
import 'rust_binding.dart';
import 'register_page.dart';
import 'graph_demo_page.dart'; // 登录成功后要跳转的页面

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _emailCtrl = TextEditingController();
  final _passCtrl = TextEditingController();
  String _error = "";

  /// 点击登录
  Future<void> _onLoginPressed() async {
    final email = _emailCtrl.text.trim();
    final pass = _passCtrl.text.trim();
    setState(() => _error = ""); // 清空旧错误

    try {
      // 异步调用
      final jsonStr = await RustBinding().loginUserFFI(email, pass);
      final data = jsonDecode(jsonStr);

      if (data['error'] != null) {
        // 后端返回 {"error": "..."}
        setState(() => _error = "Login failed: ${data['error']}");
      } else {
        // 登录成功 => 跳转到画布
        Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const GraphDemoPage()),
        );
      }
    } catch (e) {
      // 可能 JSON decode 出错等
      setState(() => _error = "Login parse error: $e");
    }
  }

  /// 点击注册
  void _onRegisterPressed() {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => const RegisterPage()),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("Login Page")),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            TextField(
              controller: _emailCtrl,
              decoration: const InputDecoration(labelText: "Email"),
            ),
            TextField(
              controller: _passCtrl,
              decoration: const InputDecoration(labelText: "Password"),
              obscureText: true,
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _onLoginPressed,
              child: const Text("Login"),
            ),
            const SizedBox(height: 8),
            TextButton(
              onPressed: _onRegisterPressed,
              child: const Text("Register"),
            ),
            if (_error.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: Text(_error, style: const TextStyle(color: Colors.red)),
              ),
          ],
        ),
      ),
    );
  }
}
