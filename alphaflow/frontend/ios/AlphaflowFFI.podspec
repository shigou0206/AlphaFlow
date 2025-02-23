Pod::Spec.new do |s|
  s.name             = 'AlphaflowFFI'
  s.version          = '0.0.1'
  s.summary          = 'A Rust dynamic library for iOS'
  s.description      = <<-DESC
                       Embedded Rust .dylib for alphaflow via vendored_libraries.
                       DESC
  s.homepage         = 'https://example.com'
  s.license          = { :type => 'MIT' }
  s.author           = { 'You' => 'liuzhihao0206@gmail.com' }
  s.platform         = :ios, '14.0'
  s.source           = { :path => '.' }
  s.vendored_libraries = 'Vendored/libffi_interface.a'
  s.preserve_paths = 'Vendored/libffi_interface.a'
  s.libraries = 'sqlite3'
end
