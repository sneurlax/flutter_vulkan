#ifndef COMMON_H
#define COMMON_H

#ifdef __APPLE__
    #include <TargetConditionals.h>
    #define _IS_IOS_ 1
#endif

#ifdef __linux__
    #define _IS_LINUX_ 1
#endif

#ifdef _IS_LINUX_
#include <flutter_linux/flutter_linux.h>
#include "../linux/include/fl_my_texture.h"
#include <iostream>
#include <gtk/gtk.h>
#include <glib-object.h>
#define LOGD(TAG,...) printf(TAG),printf(" "),printf(__VA_ARGS__),printf("\n");fflush(stdout);

#define FFI_PLUGIN_EXPORT __attribute__((visibility("default"))) __attribute__((used))

typedef struct flutter_vulkan_plugin_context
{
    FlTextureRegistrar *texture_registrar;
    FlMyTexture *myTexture;
    FlTexture *texture;
    int width;
    int height;
} VulkanPluginContext;
static VulkanPluginContext ctx_f = {
        nullptr,
        nullptr,
        nullptr,
        0,
        0};

#endif

#ifdef _IS_IOS_
#include <iostream>
#include <cstdint>
#define LOGD(TAG,...) printf(TAG),printf(" "),printf(__VA_ARGS__),printf("\n");fflush(stdout);

#define FFI_PLUGIN_EXPORT __attribute__((visibility("default"))) __attribute__((used))

typedef struct flutter_vulkan_plugin_context
{
    uint8_t *buffer;
    int width;
    int height;
    void (*markFrameAvailable)(void *registryRef);
    void *registryRef;
} VulkanPluginContext;
static VulkanPluginContext ctx_f = {
        nullptr,
        0,
        0,
        nullptr,
        nullptr};

#endif

#endif // COMMON_H
