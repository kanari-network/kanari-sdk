#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint kanari_crypto.podspec` to validate before publishing.
#
require 'yaml'

pubspec = YAML.load_file(File.join(__dir__, '..', 'pubspec.yaml'))

Pod::Spec.new do |s|
  s.name             = 'kanari_crypto'
  s.version          = pubspec['version']
  s.summary          = pubspec['description']
  s.description      = <<-DESC
A Flutter package for Kanari cryptographic functionalities using Rust.
                       DESC
  s.homepage         = pubspec['homepage'] || 'http://example.com'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Your Company' => 'email@example.com' }
  s.source           = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.dependency 'Flutter'
  s.platform = :ios, '12.0'

  # Flutter.framework does not contain a i386 slice.
  s.pod_target_xcconfig = { 
    'DEFINES_MODULE' => 'YES', 
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
    'OTHER_LDFLAGS' => '-lrust',
    'LIBRARY_SEARCH_PATHS' => '$(PODS_TARGET_SRCROOT)'
  }
  s.swift_version = '5.0'

  # Include the static library
  s.static_framework = true
  s.vendored_libraries = 'librust.a'
end
