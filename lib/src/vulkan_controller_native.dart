import 'dart:ffi' as ffi;
import 'dart:io';

import 'flutter_vulkan_api.dart';
import 'flutter_vulkan_ffi.dart';
import 'vulkan_renderer.dart';

VulkanRenderer createRenderer() {
  final ffi.DynamicLibrary nativeLib;
  if (Platform.isAndroid) {
    nativeLib = ffi.DynamicLibrary.open('libflutter_vulkan_plugin.so');
  } else if (Platform.isWindows) {
    nativeLib = ffi.DynamicLibrary.open('flutter_vulkan_plugin.dll');
  } else {
    nativeLib = ffi.DynamicLibrary.process();
  }
  return FlutterVulkanFfi.fromLookup(nativeLib.lookup);
}

FlutterVulkan createVulkanPlugin() => FlutterVulkan();
