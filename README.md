# flutter_vulkan

Flutter plugin for GPU-accelerated shader rendering. Runs GLSL fragment shaders
(including ShaderToy-compatible shaders) on all major platforms.

| Platform | Backend | Status |
|----------|---------|--------|
| Linux    | Vulkan  | ✓      |
| macOS    | MoltenVK | ✓    |
| iOS      | MoltenVK | ✓    |
| Android  | Vulkan  | ✓      |
| Web      | WebGL2  | ✓      |

## Usage

```dart
import 'package:flutter_vulkan/flutter_vulkan.dart';

// Initialize once at startup
VulkanController().initializeVulkan();

// Create a surface and render
final textureId = await VulkanController().vulkanPlugin.createSurface(width, height);
VulkanController().renderer.setShaderToy(glslSource);
VulkanController().renderer.startThread();

// Display with the VulkanTexture widget
VulkanTexture(id: textureId)
```

## Platform requirements

- **Linux**: Vulkan driver, `libshaderc` (bundled)
- **macOS 11+**: arm64 only (Apple Silicon)
- **iOS 13+**: arm64 only
- **Android**: `minSdk 24`, arm64-v8a, Vulkan-capable device
- **Web**: Browser with WebGL2 support

## License

BSD 3-Clause. See [LICENSE](LICENSE).
