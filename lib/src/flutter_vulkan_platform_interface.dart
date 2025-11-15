import 'package:plugin_platform_interface/plugin_platform_interface.dart';

import 'flutter_vulkan_method_channel.dart';

abstract class FlutterVulkanPlatform extends PlatformInterface {
  FlutterVulkanPlatform() : super(token: _token);

  static final Object _token = Object();

  static FlutterVulkanPlatform _instance = MethodChannelFlutterVulkan();

  int textureId = -1;

  static FlutterVulkanPlatform get instance => _instance;

  static set instance(FlutterVulkanPlatform instance) {
    PlatformInterface.verifyToken(instance, _token);
    _instance = instance;
  }

  Future<int> createSurface(int width, int height) async {
    throw UnimplementedError('createSurface() has not been implemented.');
  }
}
