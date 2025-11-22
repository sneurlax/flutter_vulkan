#ifndef FLUTTER_MY_TEXTURE_H
#define FLUTTER_MY_TEXTURE_H

#include <gtk/gtk.h>
#include <glib-object.h>
#include "flutter_vulkan/flutter_vulkan_plugin.h"
#include <flutter_linux/flutter_linux.h>

G_DECLARE_FINAL_TYPE(FlMyTexture,
                     fl_my_texture,
                     FL,
                     MY_TEXTURE,
                     FlPixelBufferTexture)

struct _FlMyTexture
{
    FlPixelBufferTexture parent_instance;
    uint32_t width;
    uint32_t height;
    uint8_t *buffer;
};

#define FLUTTER_VULKAN_PLUGIN(obj)                                     \
  (G_TYPE_CHECK_INSTANCE_CAST((obj), flutter_vulkan_plugin_get_type(), \
                              FlutterVulkanPlugin))

struct _FlutterVulkanPlugin
{
  GObject parent_instance;
  FlTextureRegistrar *texture_registrar;
  FlMyTexture *myTexture;
  FlTexture *texture;
  FlView *fl_view;
};

FlMyTexture *fl_my_texture_new(uint32_t width, uint32_t height);

#endif // FLUTTER_MY_TEXTURE_H
