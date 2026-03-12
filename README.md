# flutter_vulkan

Flutter plugin for GPU-accelerated shader rendering with Vulkan.  Runs GLSL fragment shaders (including ShaderToy-compatible shaders) on all major platforms.

| Platform | Backend  | Status |
|----------|----------|--------|
| Android  | Vulkan   | ✓      |
| iOS      | MoltenVK | ✓      |
| Linux    | Vulkan   | ✓      |
| macOS    | MoltenVK | ✓      |
| Web      | WebGL2   | ✓      |
| Windows  | Vulkan   | ✓      |

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

## Requirements

- **Flutter** 3.38.5 or newer
- **Rust** 1.94.0 or newer

## Platform requirements

- **Android**: `minSdk 24`, arm64-v8a, Vulkan-capable device
- **iOS 13+**: arm64 only
- **Linux**: Vulkan driver, `libshaderc` (bundled)
- **macOS 11+**: arm64 only (Apple Silicon)
- **Web**: Browser with WebGL2 support
- **Windows 10+**: Vulkan-capable GPU and driver; Visual Studio Build Tools with the "Desktop development with C++" workload (MSVC, CMake, Windows SDK)
- **WSL2**: `ninja-build build-essential pkg-config libgtk-3-dev clang`

## License

MIT. See [LICENSE](LICENSE).
