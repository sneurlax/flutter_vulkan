#ifndef COMMON_H
#define COMMON_H

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

#endif // COMMON_H
