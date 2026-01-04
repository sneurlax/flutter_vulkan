import 'dart:ffi' as ffi;
import 'dart:io';

import 'flutter_vulkan_api.dart';
import 'flutter_vulkan_ffi.dart';

class VulkanController {
  static VulkanController? _instance;

  factory VulkanController() => _instance ??= VulkanController._();

  VulkanController._();

  late ffi.DynamicLibrary nativeLib;
  late final FlutterVulkan vulkanPlugin;
  late final FlutterVulkanFfi vulkanFFI;

  initializeVulkan() {
    nativeLib = Platform.isAndroid
        ? ffi.DynamicLibrary.open('libflutter_vulkan_plugin.so')
        : ffi.DynamicLibrary.process();
    vulkanFFI = FlutterVulkanFfi.fromLookup(nativeLib.lookup);
    vulkanPlugin = FlutterVulkan();
  }
}
