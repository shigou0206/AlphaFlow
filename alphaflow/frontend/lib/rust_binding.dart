import 'package:flutter/services.dart';

/// RustBinding: 提供 createUserFFI, loginUserFFI, getUserFFI 三个异步函数，
/// 以及 runMigrationsFFI，内部通过 MethodChannel 调用 iOS 原生 -> Rust 静态库
class RustBinding {
  static final RustBinding _instance = RustBinding._internal();
  factory RustBinding() => _instance;
  RustBinding._internal();

  bool _initialized = false;
  late MethodChannel _channel;

  /// 初始化：设置 MethodChannel
  void init() {
    if (_initialized) return;
    // 与 iOS AppDelegate.swift 里保持一致
    _channel = const MethodChannel('rust_bridge_channel');
    _initialized = true;
  }

  /// create_user_ffi
  /// 传入 userId, email, pass, role => 返回 JSON 字符串
  Future<String> createUserFFI(
      String userId, String email, String pass, String role) async {
    final args = {
      'userId': userId,
      'email': email,
      'pass': pass,
      'role': role,
    };
    final result = await _channel.invokeMethod<String>('createUser', args);
    return result ?? '{"error":"create_user_ffi => null"}';
  }

  /// login_user_ffi
  /// 传入 email, pass => 返回 JSON
  Future<String> loginUserFFI(String email, String pass) async {
    final args = {
      'email': email,
      'pass': pass,
    };
    final result = await _channel.invokeMethod<String>('loginUser', args);
    return result ?? '{"error":"login_user_ffi => null"}';
  }

  /// get_user_by_id_ffi
  /// 传入 userId => 返回 JSON
  Future<String> getUserFFI(String userId) async {
    final args = {
      'userId': userId,
    };
    final result = await _channel.invokeMethod<String>('getUser', args);
    return result ?? '{"error":"get_user_by_id_ffi => null"}';
  }

  /// 新增：run_migrations_ffi
  /// iOS 端 AppDelegate.swift 中的 case "runMigrations" -> MyRustWrapper.runMigrations()
  Future<String> runMigrationsFFI() async {
    // 不需要参数 => 给个空 map or null
    final result = await _channel.invokeMethod<String>('runMigrations');
    return result ?? '{"info":"run_migrations => null"}';
  }
}
