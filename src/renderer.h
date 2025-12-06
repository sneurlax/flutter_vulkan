#ifndef RENDERER_H
#define RENDERER_H

#include "common.h"
#include "shader.h"
#include "vulkan_context.h"

#include <thread>
#include <mutex>
#include <memory>
#include <vector>
#include <string>

class Renderer {
public:
    Renderer(VulkanPluginContext *textureStruct);
    ~Renderer();

    void stop();
    void loop();

    std::string setShader(bool isContinuous, const char *vertexSource, const char *fragmentSource);
    std::string setShaderToy(const char *fragmentSource);

    inline std::string getCompileError() { return compileError; }
    inline Shader *getShader() { return shader.get(); }
    inline bool isLooping() { return loopRunning; }
    inline double getFrameRate() { return frameRate; }
    inline void setNewTextureMsg() { msg.push_back(MSG_NEW_TEXTURE); }

    VulkanContext *getVulkanContext() { return &vulkanCtx; }

private:
    VulkanPluginContext *self;
    VulkanContext vulkanCtx;
    std::mutex mutex;
    double frameRate;

    std::string compileError;
    std::unique_ptr<Shader> shader;
    bool newShaderIsContinuous;
    std::string newShaderFragmentSource;
    std::string newShaderVertexSource;

    bool isShaderToy;
    bool loopRunning;

    enum RenderThreadMessage : int {
        MSG_NONE = 0,
        MSG_STOP_RENDERER,
        MSG_NEW_SHADER,
        MSG_NEW_TEXTURE,
    };
    std::vector<RenderThreadMessage> msg;
};

#endif // RENDERER_H
