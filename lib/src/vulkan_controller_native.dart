import 'dart:ffi' as ffi;
import 'dart:io';

import 'flutter_vulkan_api.dart';
import 'flutter_vulkan_ffi.dart';
import 'vulkan_renderer.dart';

VulkanRenderer createRenderer() {
  final nativeLib = Platform.isAndroid
      ? ffi.DynamicLibrary.open('libflutter_vulkan_plugin.so')
      : ffi.DynamicLibrary.process();
  return FlutterVulkanFfi.fromLookup(nativeLib.lookup);
}

FlutterVulkan createVulkanPlugin() => FlutterVulkan();
