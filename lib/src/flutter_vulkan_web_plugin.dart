import 'flutter_vulkan_platform_interface.dart';
import 'web/webgl_renderer.dart';

class FlutterVulkanWeb extends FlutterVulkanPlatform {
  FlutterVulkanWeb._();

  static void registerWith(Object registrar) {
    FlutterVulkanPlatform.instance = FlutterVulkanWeb._();
  }

  @override
  Future<int> createSurface(int width, int height) async {
    return WebGLRenderer.instance.createSurface(width, height);
  }
}
