#include "include/fl_my_texture.h"

#include <iostream>
#include <cstring>

G_DEFINE_TYPE(FlMyTexture,
              fl_my_texture,
              fl_pixel_buffer_texture_get_type())

static gboolean fl_my_texture_copy_pixels(FlPixelBufferTexture *texture,
                                           const uint8_t **out_buffer,
                                           uint32_t *width,
                                           uint32_t *height,
                                           GError **error)
{
  FlMyTexture* f = FL_MY_TEXTURE(texture);
  *out_buffer = f->buffer;
  *width = f->width;
  *height = f->height;
  return TRUE;
}

FlMyTexture *fl_my_texture_new(uint32_t width, uint32_t height)
{
  auto r = FL_MY_TEXTURE(g_object_new(fl_my_texture_get_type(), nullptr));
  r->width = width;
  r->height = height;
  r->buffer = new uint8_t[width * height * 4]();
  return r;
}

static void fl_my_texture_dispose(GObject *object)
{
  FlMyTexture *self = FL_MY_TEXTURE(object);
  delete[] self->buffer;
  self->buffer = nullptr;
  G_OBJECT_CLASS(fl_my_texture_parent_class)->dispose(object);
}

static void fl_my_texture_class_init(FlMyTextureClass *klass)
{
  FL_PIXEL_BUFFER_TEXTURE_CLASS(klass)->copy_pixels =
      fl_my_texture_copy_pixels;
  G_OBJECT_CLASS(klass)->dispose = fl_my_texture_dispose;
}

static void fl_my_texture_init(FlMyTexture *self)
{
  self->buffer = nullptr;
}
