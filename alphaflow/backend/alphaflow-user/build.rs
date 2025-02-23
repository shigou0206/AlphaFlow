fn main() {
  #[cfg(feature = "dart")]
  {
    alphaflow_codegen::protobuf_file::dart_gen(env!("CARGO_PKG_NAME"));
    alphaflow_codegen::dart_event::gen(env!("CARGO_PKG_NAME"));
  }

  #[cfg(feature = "tauri_ts")]
  {
    alphaflow_codegen::ts_event::gen(env!("CARGO_PKG_NAME"), alphaflow_codegen::Project::Tauri);
    alphaflow_codegen::protobuf_file::ts_gen(
      env!("CARGO_PKG_NAME"),
      env!("CARGO_PKG_NAME"),
      alphaflow_codegen::Project::Tauri,
    );
    alphaflow_codegen::ts_event::gen(env!("CARGO_PKG_NAME"), alphaflow_codegen::Project::TauriApp);
    alphaflow_codegen::protobuf_file::ts_gen(
      env!("CARGO_PKG_NAME"),
      env!("CARGO_PKG_NAME"),
      alphaflow_codegen::Project::TauriApp,
    );
  }
}
