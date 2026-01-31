import '../flutter_vulkan_api.dart';
import '../vulkan_renderer.dart';
import 'webgl_renderer.dart';

VulkanRenderer createRenderer() => WebGLRenderer.instance;

FlutterVulkan createVulkanPlugin() => FlutterVulkan();
