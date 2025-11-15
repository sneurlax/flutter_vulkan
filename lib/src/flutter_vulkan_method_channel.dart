import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'flutter_vulkan_platform_interface.dart';

class MethodChannelFlutterVulkan extends FlutterVulkanPlatform {
  @visibleForTesting
  final methodChannel = const MethodChannel('flutter_vulkan_plugin');

  @override
  Future<int> createSurface(int width, int height) async {
    int? textureId;
    try {
      textureId = await methodChannel.invokeMethod<int>('createSurface', {
        'width': width,
        'height': height,
      });
    } on PlatformException catch (e) {
      debugPrint(e.toString());
      return -1;
    }
    return textureId ?? -1;
  }
}
