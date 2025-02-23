import UIKit
import Flutter

@UIApplicationMain
@objc class AppDelegate: FlutterAppDelegate {
    override func application(
      _ application: UIApplication,
      didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {

        RustDB.initializePool()

        let controller = window?.rootViewController as! FlutterViewController
        let channel = FlutterMethodChannel(name: "rust_bridge_channel",
                                           binaryMessenger: controller.binaryMessenger)

        channel.setMethodCallHandler { (call, result) in
            switch call.method {
            case "createUser":
                if let args = call.arguments as? [String:String],
                   let userId = args["userId"],
                   let email  = args["email"],
                   let pass   = args["pass"],
                   let role   = args["role"] {
                    let json = MyRustWrapper.createUser(userId: userId, email: email, pass: pass, role: role)
                    result(json)
                } else {
                    result(FlutterError(code: "BAD_ARGS", message: "Missing userId/email/pass/role", details: nil))
                }

            case "getUser":
                if let args = call.arguments as? [String:String],
                   let userId = args["userId"] {
                    let json = MyRustWrapper.getUserById(userId: userId)
                    result(json)
                } else {
                    result(FlutterError(code: "BAD_ARGS", message: "No userId", details: nil))
                }

            case "loginUser":
                if let args = call.arguments as? [String:String],
                   let email = args["email"],
                   let pass  = args["pass"] {
                    let json = MyRustWrapper.loginUser(email: email, pass: pass)
                    result(json)
                } else {
                    result(FlutterError(code: "BAD_ARGS", message: "No email/pass", details: nil))
                }

            default:
                result(FlutterMethodNotImplemented)
            }
        }

        return super.application(application, didFinishLaunchingWithOptions: launchOptions)
    }
}