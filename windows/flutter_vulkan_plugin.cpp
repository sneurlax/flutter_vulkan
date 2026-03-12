#include "include/flutter_vulkan/flutter_vulkan_plugin.h"

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>
#include <flutter/texture_registrar.h>

#include <memory>
#include <string>

// ---------------------------------------------------------------------------
// Rust FFI – symbols exported by the Rust static library linked into this DLL
// ---------------------------------------------------------------------------
extern "C" {
    void createRenderer(uint8_t *buffer, int width, int height);
    void deleteRenderer();
    void *getRenderer();
    void stopThread();
    void setFrameCallback(void (*callback)(void *), void *user_data);
}

namespace flutter_vulkan {

// ---------------------------------------------------------------------------
// Frame callback data – lives as long as the plugin instance.
// ---------------------------------------------------------------------------
struct FrameCallbackData {
    flutter::TextureRegistrar *registrar;
    int64_t texture_id;
};

static FrameCallbackData *g_frame_cb_data = nullptr;

// Called from the Rust render thread after each frame has been written into
// the pixel buffer.  TextureRegistrar::MarkTextureFrameAvailable() is
// thread-safe in the Flutter Windows embedding – it posts internally to the
// raster thread – so we can call it directly here.
static void on_frame_available(void *user_data)
{
    auto *data = static_cast<FrameCallbackData *>(user_data);
    data->registrar->MarkTextureFrameAvailable(data->texture_id);
}

// ---------------------------------------------------------------------------
// Plugin class
// ---------------------------------------------------------------------------
class FlutterVulkanPlugin : public flutter::Plugin {
public:
    static void RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar);

    FlutterVulkanPlugin(flutter::PluginRegistrarWindows *registrar);
    ~FlutterVulkanPlugin() override;

private:
    void HandleMethodCall(
        const flutter::MethodCall<flutter::EncodableValue> &method_call,
        std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);

    flutter::PluginRegistrarWindows *registrar_;
    flutter::TextureRegistrar *texture_registrar_;

    int64_t texture_id_ = -1;
    std::unique_ptr<flutter::TextureVariant> texture_variant_;
    FlutterDesktopPixelBuffer pixel_buffer_desc_{};
    uint8_t *pixel_buffer_ = nullptr;
    int width_ = 0;
    int height_ = 0;
};

// static
void FlutterVulkanPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar)
{
    auto channel =
        std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
            registrar->messenger(), "flutter_vulkan_plugin",
            &flutter::StandardMethodCodec::GetInstance());

    auto plugin = std::make_unique<FlutterVulkanPlugin>(registrar);

    channel->SetMethodCallHandler(
        [plugin_ptr = plugin.get()](const auto &call, auto result) {
            plugin_ptr->HandleMethodCall(call, std::move(result));
        });

    registrar->AddPlugin(std::move(plugin));
}

FlutterVulkanPlugin::FlutterVulkanPlugin(flutter::PluginRegistrarWindows *registrar)
    : registrar_(registrar),
      texture_registrar_(registrar->texture_registrar()) {}

FlutterVulkanPlugin::~FlutterVulkanPlugin()
{
    if (texture_id_ != -1) {
        texture_registrar_->UnregisterTexture(texture_id_, nullptr);
    }
    delete[] pixel_buffer_;
    pixel_buffer_ = nullptr;
    delete g_frame_cb_data;
    g_frame_cb_data = nullptr;
}

void FlutterVulkanPlugin::HandleMethodCall(
    const flutter::MethodCall<flutter::EncodableValue> &method_call,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result)
{
    if (method_call.method_name() == "createSurface") {
        const auto *args =
            std::get_if<flutter::EncodableMap>(method_call.arguments());
        int width = 0, height = 0;
        if (args) {
            auto w = args->find(flutter::EncodableValue(std::string("width")));
            auto h = args->find(flutter::EncodableValue(std::string("height")));
            if (w != args->end())
                width = std::get<int>(w->second);
            if (h != args->end())
                height = std::get<int>(h->second);
        }

        if (width == 0 || height == 0) {
            result->Error(
                "100",
                "MethodCall createSurface() called without passing "
                "width and height parameters!");
            return;
        }

        // Clean up any previous surface / render thread.
        if (texture_id_ != -1) {
            texture_registrar_->UnregisterTexture(texture_id_, nullptr);
            texture_id_ = -1;
            if (getRenderer() != nullptr)
                stopThread();
        }
        delete[] pixel_buffer_;

        width_ = width;
        height_ = height;
        pixel_buffer_ = new uint8_t[width * height * 4]();

        // Describe the pixel buffer to Flutter.
        pixel_buffer_desc_.buffer = pixel_buffer_;
        pixel_buffer_desc_.width = static_cast<size_t>(width);
        pixel_buffer_desc_.height = static_cast<size_t>(height);
        pixel_buffer_desc_.release_callback = nullptr;
        pixel_buffer_desc_.release_context = nullptr;

        texture_variant_ = std::make_unique<flutter::TextureVariant>(
            flutter::PixelBufferTexture(
                [this](size_t /*w*/, size_t /*h*/)
                    -> const FlutterDesktopPixelBuffer * {
                    return &pixel_buffer_desc_;
                }));

        texture_id_ =
            texture_registrar_->RegisterTexture(texture_variant_.get());
        texture_registrar_->MarkTextureFrameAvailable(texture_id_);

        // Start the Rust renderer.
        createRenderer(pixel_buffer_, width, height);

        // Register the frame callback so Rust can notify Flutter.
        delete g_frame_cb_data;
        g_frame_cb_data =
            new FrameCallbackData{texture_registrar_, texture_id_};
        setFrameCallback(on_frame_available, g_frame_cb_data);

        result->Success(flutter::EncodableValue(texture_id_));
    } else {
        result->NotImplemented();
    }
}

}  // namespace flutter_vulkan

// ---------------------------------------------------------------------------
// C entry-point called by the Flutter Windows plugin loader
// ---------------------------------------------------------------------------
void FlutterVulkanPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar)
{
    flutter_vulkan::FlutterVulkanPlugin::RegisterWithRegistrar(
        flutter::PluginRegistrarManager::GetInstance()
            ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
