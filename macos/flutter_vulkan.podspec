Pod::Spec.new do |s|
  s.name             = 'flutter_vulkan'
  s.version          = '0.0.1'
  s.summary          = 'Flutter plugin to bind a Texture widget to a Vulkan context via MoltenVK.'
  s.description      = <<-DESC
  Flutter plugin for Vulkan-based shader rendering using MoltenVK on macOS.
                       DESC
  s.homepage         = 'https://github.com/sneurlax/flutter_vulkan'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'sneurlax' => 'sneurlax@gmail.com' }
  s.source           = { :path => '.' }

  s.source_files = 'Classes/**/*.swift', 'Classes/include/**/*.h', 'src/**/*.{cpp,h}'
  s.public_header_files = 'Classes/include/flutter_vulkan_bridge.h'

  s.dependency 'FlutterMacOS'
  s.platform = :osx, '11.0'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'CLANG_CXX_LANGUAGE_STANDARD' => 'c++17',
    'GCC_PREPROCESSOR_DEFINITIONS' => '$(inherited) _IS_MACOS_=1',
    'HEADER_SEARCH_PATHS' => '$(PODS_TARGET_SRCROOT)/src $(PODS_TARGET_SRCROOT)/../third_party/shaderc/include $(PODS_TARGET_SRCROOT)/../third_party/vulkan/include $(PODS_TARGET_SRCROOT)/Classes/include',
    'OTHER_LDFLAGS' => '-lc++ -lMoltenVK -lshaderc_combined',
    'LIBRARY_SEARCH_PATHS' => '$(PODS_TARGET_SRCROOT)/Libraries',
    'EXCLUDED_ARCHS[sdk=macosx*]' => 'x86_64',
    'SWIFT_INCLUDE_PATHS' => '$(PODS_TARGET_SRCROOT)/Classes/include',
  }

  s.user_target_xcconfig = {
    'EXCLUDED_ARCHS[sdk=macosx*]' => 'x86_64',
  }

  s.swift_version = '5.0'

  # Vendored static libraries
  s.vendored_libraries = 'Libraries/libMoltenVK.a', 'Libraries/libshaderc_combined.a'

  # System frameworks required by MoltenVK
  s.frameworks = 'Metal', 'Foundation', 'QuartzCore', 'CoreGraphics',
                 'IOSurface', 'IOKit', 'AppKit', 'CoreVideo'
end
