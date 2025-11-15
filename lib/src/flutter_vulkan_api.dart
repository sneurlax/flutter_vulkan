import 'flutter_vulkan_platform_interface.dart';

class FlutterVulkan {
  Future<int> createSurface(int width, int height) {
    return FlutterVulkanPlatform.instance.createSurface(width, height);
  }
}
