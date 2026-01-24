#include "common.h"
#if FLUTTER_VULKAN_SIMULATOR_STUB

#include <cstdint>
#include <cstddef>

extern "C" void createRenderer(VulkanPluginContext *) {}
extern "C" void deleteRenderer() {}
extern "C" void *getRenderer() { return NULL; }
extern "C" bool rendererStatus() { return false; }
extern "C" void getTextureSize(int32_t *width, int32_t *height) { *width = -1; *height = -1; }
extern "C" void startThread() {}
extern "C" void stopThread() {}
extern "C" const char *setShader(bool, const char *, const char *) { return ""; }
extern "C" const char *setShaderToy(const char *) { return ""; }
extern "C" const char *getVertexShader() { return ""; }
extern "C" const char *getFragmentShader() { return ""; }
extern "C" void addShaderToyUniforms() {}
extern "C" void setMousePosition(double, double, double, double, double, double) {}
extern "C" double getFPS() { return -1.0; }

#endif // FLUTTER_VULKAN_SIMULATOR_STUB
