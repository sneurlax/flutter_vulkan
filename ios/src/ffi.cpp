#include "common.h"

#include "ffi.h"
#include "renderer.h"
#include "uniform_queue.h"
#include "sampler2d.h"

Renderer *renderer = nullptr;

#define LOG_TAG_FFI "NATIVE FFI"

extern "C" void deleteRenderer() {
    if (renderer != nullptr) {
        if (renderer->isLooping()) {
            while (bool b = renderer->isLooping()) renderer->stop();
        }
        delete renderer;
        renderer = nullptr;
    }
}

extern "C" void createRenderer(VulkanPluginContext *textureStruct) {
    deleteRenderer();
    renderer = new Renderer(textureStruct);
}

extern "C" void *getRenderer() { return (void *)renderer; }

extern "C" FFI_PLUGIN_EXPORT bool rendererStatus() {
    if (renderer == nullptr) return false;
    return true;
}

extern "C" FFI_PLUGIN_EXPORT void getTextureSize(int32_t *width, int32_t *height) {
    if (renderer == nullptr || renderer->getShader() == nullptr) {
        *width = -1;
        *height = -1;
        return;
    }
    *width = renderer->getShader()->getWidth();
    *height = renderer->getShader()->getHeight();
}

extern "C" FFI_PLUGIN_EXPORT void
startThread() {
    if (renderer == nullptr) {
        LOGD(LOG_TAG_FFI, "startThread: Texture not yet created!");
        return;
    }
    std::thread loopThread = std::thread([&]() {
        renderer->loop();
    });
    loopThread.detach();
}

extern "C" FFI_PLUGIN_EXPORT void
stopThread() {
    if (renderer == nullptr) {
        LOGD(LOG_TAG_FFI, "stopThread: Renderer not yet created!");
        return;
    }
    renderer->stop();
    while (renderer->isLooping());
}

std::string compileError;
extern "C" FFI_PLUGIN_EXPORT const char *
setShader(bool isContinuous,
          const char *vertexShader, const char *fragmentShader) {
    if (renderer == nullptr) {
        LOGD(LOG_TAG_FFI, "setShader: Renderer not yet created!");
        return "";
    }
    compileError = renderer->setShader(
            isContinuous,
            vertexShader,
            fragmentShader);
    return compileError.c_str();
}

extern "C" FFI_PLUGIN_EXPORT const char *
setShaderToy(const char *fragmentShader) {
    if (renderer == nullptr) {
        LOGD(LOG_TAG_FFI, "setShaderToy: Renderer not yet created!");
        return "";
    }
    compileError = renderer->setShaderToy(fragmentShader);
    return compileError.c_str();
}

extern "C" FFI_PLUGIN_EXPORT const char *
getVertexShader() {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return "";
    return renderer->getShader()->vertexSource.c_str();
}

extern "C" FFI_PLUGIN_EXPORT const char *
getFragmentShader() {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return "";
    return renderer->getShader()->fragmentSource.c_str();
}

extern "C" FFI_PLUGIN_EXPORT void
addShaderToyUniforms() {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return;
    renderer->getShader()->addShaderToyUniforms();
}

extern "C" FFI_PLUGIN_EXPORT void
setMousePosition(
    double posX, double posY, double posZ, double posW,
    double textureWidgetWidth, double textureWidgetHeight) {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return;
    double textureWidth = renderer->getShader()->getWidth();
    double textureHeight = renderer->getShader()->getHeight();
    double arH = textureWidth / textureWidgetWidth;
    double arV = textureHeight / textureWidgetHeight;
    posX *= arH;
    posY *= arV;
    posZ *= arH;
    posW *= arV;
    posY = textureHeight - posY;
    posW = -textureHeight - posW;
    auto mouse = vec4{
        (float)posX,
        (float)posY,
        (float)posZ,
        (float)posW};
    renderer->getShader()->getUniforms().setUniformValue(
            std::string("iMouse"),
            (void *)(&mouse));
}

extern "C" FFI_PLUGIN_EXPORT double
getFPS() {
    if (renderer == nullptr || !renderer->isLooping())
        return -1.0;
    return renderer->getFrameRate();
}

extern "C" FFI_PLUGIN_EXPORT bool
addUniform(const char *name, UniformType type, void *val) {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return false;
    renderer->getShader()->getUniforms().addUniform(name, type, val);
    return true;
}

extern "C" FFI_PLUGIN_EXPORT bool
removeUniform(const char *name) {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return false;
    return renderer->getShader()->getUniforms().removeUniform(name);
}

extern "C" FFI_PLUGIN_EXPORT bool
setUniform(const char *name, void *val) {
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return false;
    return renderer->getShader()->getUniforms().setUniformValue(name, val);
}

extern "C" FFI_PLUGIN_EXPORT bool
addSampler2DUniform(const char *name, int width, int height, void *val)
{
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return false;
    Sampler2D sampler;
    sampler.add_RGBA32(width, height, (unsigned char*)val);
    bool ret = renderer->getShader()->getUniforms()
        .addUniform(name, UNIFORM_SAMPLER2D, (void*)&sampler);

    if (ret && renderer->isLooping())
        renderer->setNewTextureMsg();
    return ret;
}

extern "C" FFI_PLUGIN_EXPORT bool
replaceSampler2DUniform(const char *name, int width, int height, void *val)
{
    if (renderer == nullptr || renderer->getShader() == nullptr)
        return false;
    bool replaced = renderer->getShader()->getUniforms()
        .replaceSampler2D(name, width, height, (unsigned char *)val);

    if (replaced && renderer->isLooping())
        renderer->setNewTextureMsg();
    return replaced;
}
