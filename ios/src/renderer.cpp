#include "common.h"

#include "renderer.h"

#include <chrono>

#define LOG_TAG_RENDERER "RENDERER"
#define DEBUG true

Renderer::Renderer(VulkanPluginContext *textureStruct)
        : self(textureStruct),
          frameRate(0.0),
          shader(nullptr),
          isShaderToy(false),
          loopRunning(false)
{
    msg.push_back(MSG_NONE);

    if (!vulkanCtx.init()) {
        LOGD(LOG_TAG_RENDERER, "Failed to initialize Vulkan context!");
    }
}

Renderer::~Renderer() {
    if (shader.get() != nullptr) {
        shader.reset();
    }
    vulkanCtx.cleanup();
}

void Renderer::stop() {
    msg.push_back(MSG_STOP_RENDERER);
}

std::string Renderer::setShader(bool isContinuous,
                                const char *vertexSource,
                                const char *fragmentSource) {
    compileError = "";
    isShaderToy = false;

    newShaderFragmentSource = fragmentSource;
    newShaderVertexSource = vertexSource;
    newShaderIsContinuous = isContinuous;
    msgProcessed = false;
    msg.push_back(MSG_NEW_SHADER);
    if (loopRunning)
        while (!msgProcessed)
            std::this_thread::yield();
    return compileError;
}

std::string Renderer::setShaderToy(const char *fragmentSource) {
    compileError = "";
    isShaderToy = true;

    newShaderFragmentSource = fragmentSource;
    newShaderVertexSource = "";
    newShaderIsContinuous = true;
    msgProcessed = false;
    msg.push_back(MSG_NEW_SHADER);
    if (loopRunning)
        while (!msgProcessed)
            std::this_thread::yield();
    return compileError;
}

void Renderer::loop() {
    if (DEBUG)
        LOGD(LOG_TAG_RENDERER, "ENTERING LOOP");

    unsigned int frames = 0;
    frameRate = 0.0;
    auto startFps = std::chrono::steady_clock::now();
    auto endFps = std::chrono::steady_clock::now();
    auto startDraw = std::chrono::steady_clock::now();
    auto endDraw = std::chrono::steady_clock::now();
    std::chrono::duration<double> elapsedFps = std::chrono::duration<double>(0);
    std::chrono::duration<double> elapsedDraw = std::chrono::duration<double>(0);
    // MAX_FPS: draw 1 frame at max every 10 ms (max 100 FPS)
    double MAX_FPS = 1.0 / 100.0;
    loopRunning = true;

    RenderThreadMessage _msg;

    while (loopRunning) {
        mutex.lock();

        if (msg.size() == 0) _msg = MSG_NONE;
        else { _msg = msg.back(); msg.pop_back(); }

        switch (_msg) {
            case MSG_NEW_SHADER:
                if (shader.get() != nullptr)
                    shader.reset();
                shader = std::make_unique<Shader>(self, &vulkanCtx);
                shader->setShadersText(newShaderVertexSource, newShaderFragmentSource);
                shader->setShadersSize(self->width, self->height);
                shader->setIsContinuous(newShaderIsContinuous);

                if (isShaderToy)
                    compileError = shader->initShaderToy();
                else
                    compileError = shader->initShader();
                msgProcessed = true;
                break;

            case MSG_NEW_TEXTURE:
                if (shader != nullptr) {
                    shader->refreshTextures();
                }
                break;

            case MSG_STOP_RENDERER:
                loopRunning = false;
                break;

            default:
                if (shader == nullptr || !shader->isContinuous())
                    break;

                elapsedFps = endFps - startFps;
                elapsedDraw = endDraw - startDraw;

                if (elapsedDraw.count() >= MAX_FPS) {
                    frames++;
                    shader->drawFrame();
                    startDraw = std::chrono::steady_clock::now();
                }
                endDraw = std::chrono::steady_clock::now();

                // update frameRate every second
                if (elapsedFps.count() >= 1.0) {
                    frameRate = (double)frames * 0.5 + frameRate * 0.5;
                    frames = 0;
                    startFps = std::chrono::steady_clock::now();
                }
                endFps = std::chrono::steady_clock::now();
                break;
        }

        mutex.unlock();
    }
    loopRunning = false;
}
