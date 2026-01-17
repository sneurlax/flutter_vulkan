#ifndef FLUTTER_VULKAN_BRIDGE_H
#define FLUTTER_VULKAN_BRIDGE_H

#include <stdint.h>
#include <stdbool.h>

typedef struct flutter_vulkan_plugin_context {
    uint8_t *buffer;
    int width;
    int height;
    void (*markFrameAvailable)(void *registryRef);
    void *registryRef;
} VulkanPluginContext;

#ifdef __cplusplus
extern "C" {
#endif

void createRenderer(VulkanPluginContext *textureStruct);
void deleteRenderer(void);
void *getRenderer(void);

bool rendererStatus(void);
void getTextureSize(int32_t *width, int32_t *height);
void startThread(void);
void stopThread(void);
const char *setShader(bool isContinuous, const char *vertexShader, const char *fragmentShader);
const char *setShaderToy(const char *fragmentShader);
const char *getVertexShader(void);
const char *getFragmentShader(void);
void addShaderToyUniforms(void);
void setMousePosition(double posX, double posY, double posZ, double posW,
                      double textureWidgetWidth, double textureWidgetHeight);
double getFPS(void);

#ifdef __cplusplus
}
#endif

#endif // FLUTTER_VULKAN_BRIDGE_H
