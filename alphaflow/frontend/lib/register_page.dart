import 'package:flutter/material.dart';
import 'dart:convert';
import 'rust_binding.dart';

class RegisterPage extends StatefulWidget {
  const RegisterPage({super.key});

  @override
  State<RegisterPage> createState() => _RegisterPageState();
}

class _RegisterPageState extends State<RegisterPage> {
  final _userIdCtrl = TextEditingController();
  final _emailCtrl = TextEditingController();
  final _passCtrl = TextEditingController();
  final _roleCtrl = TextEditingController(text: "admin");

  String _error = "";

  /// 点击 "Register"
  Future<void> _onRegister() async {
    final userId = _userIdCtrl.text.trim();
    final email = _emailCtrl.text.trim();
    final pass = _passCtrl.text.trim();
    final role = _roleCtrl.text.trim();

    setState(() => _error = "");

    try {
      // 异步等待 createUserFFI
      final jsonStr =
          await RustBinding().createUserFFI(userId, email, pass, role);
      final data = jsonDecode(jsonStr);

      if (data["error"] != null) {
        setState(() => _error = "CreateUser error: ${data["error"]}");
      } else {
        // 创建成功 => pop 回上一个页面
        Navigator.pop(context);
      }
    } catch (e) {
      setState(() => _error = "Register parse error: $e");
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text("Register Page")),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            TextField(
              controller: _userIdCtrl,
              decoration: const InputDecoration(labelText: "User ID"),
            ),
            TextField(
              controller: _emailCtrl,
              decoration: const InputDecoration(labelText: "Email"),
            ),
            TextField(
              controller: _passCtrl,
              decoration: const InputDecoration(labelText: "Password"),
              obscureText: true,
            ),
            TextField(
              controller: _roleCtrl,
              decoration: const InputDecoration(labelText: "Role"),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _onRegister,
              child: const Text("Register"),
            ),
            if (_error.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: Text(
                  _error,
                  style: const TextStyle(color: Colors.red),
                ),
              )
          ],
        ),
      ),
    );
  }
}
