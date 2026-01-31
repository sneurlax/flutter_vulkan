import 'flutter_vulkan_api.dart';
import 'vulkan_renderer.dart';

import 'vulkan_controller_native.dart'
    if (dart.library.js_interop) 'web/vulkan_controller_web.dart';

class VulkanController {
  static VulkanController? _instance;

  factory VulkanController() => _instance ??= VulkanController._();

  VulkanController._();

  late final FlutterVulkan vulkanPlugin;
  late final VulkanRenderer renderer;

  initializeVulkan() {
    renderer = createRenderer();
    vulkanPlugin = createVulkanPlugin();
  }
}
