import '../flutter_vulkan_api.dart';
import '../vulkan_renderer.dart';
import 'wasm_renderer.dart';

VulkanRenderer createRenderer() => WasmRenderer.instance;

FlutterVulkan createVulkanPlugin() => FlutterVulkan();
