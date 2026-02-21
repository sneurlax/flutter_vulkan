#ifndef SHADER_H
#define SHADER_H

#include "common.h"
#include "vulkan_context.h"
#include "uniform_queue.h"

#include <iostream>
#include <mutex>
#include <memory>
#include <vector>
#include <string>

class Shader {
public:
    Shader(VulkanPluginContext *pluginCtx, VulkanContext *vkCtx);
    ~Shader();

    void addShaderToyUniforms();
    void setShadersSize(int w, int h);
    void setShadersText(std::string vertexSource, std::string fragmentSource);
    void setIsContinuous(bool isContinuous);
    bool isContinuous() { return _isContinuous; }
    bool isValid() { return pipelineValid; }
    int getWidth() { return width; }
    int getHeight() { return height; }

    std::string initShaderToy();
    std::string initShader();

    void drawFrame();

    UniformQueue &getUniforms() { return uniformsList; }
    void refreshTextures();

    std::string compileError;
    std::string vertexSource;
    std::string fragmentSource;

private:
    mutable std::mutex mutex_;
    VulkanPluginContext *self;
    VulkanContext *vkCtx;
    bool _isContinuous = true;
    int width = 0;
    int height = 0;
    bool pipelineValid = false;
    float startTime;

    UniformQueue uniformsList;

    // Vulkan pipeline resources
    VkRenderPass renderPass = VK_NULL_HANDLE;
    VkPipelineLayout pipelineLayout = VK_NULL_HANDLE;
    VkPipeline graphicsPipeline = VK_NULL_HANDLE;
    VkDescriptorSetLayout descriptorSetLayout = VK_NULL_HANDLE;
    VkDescriptorPool descriptorPool = VK_NULL_HANDLE;
    VkDescriptorSet descriptorSet = VK_NULL_HANDLE;

    // Offscreen rendering resources
    VkImage colorImage = VK_NULL_HANDLE;
    VkDeviceMemory colorImageMemory = VK_NULL_HANDLE;
    VkImageView colorImageView = VK_NULL_HANDLE;
    VkFramebuffer framebuffer = VK_NULL_HANDLE;

#ifdef _IS_ANDROID_
    std::vector<VkFramebuffer> swapchainFramebuffers;
#endif

    // Pixel readback
    VkBuffer stagingBuffer = VK_NULL_HANDLE;
    VkDeviceMemory stagingBufferMemory = VK_NULL_HANDLE;
    void *stagingMapped = nullptr;

    // Command buffer
    VkCommandBuffer commandBuffer = VK_NULL_HANDLE;

    // SPIR-V compilation
    std::vector<uint32_t> compileGLSLToSPIRV(const std::string &source, bool isVertex);

    // Pipeline creation
    bool createRenderPass();
    bool createDescriptorSetLayout();
    bool createPipeline(const std::vector<uint32_t> &vertSpirv,
                        const std::vector<uint32_t> &fragSpirv);
    bool createOffscreenResources();
    bool createStagingBuffer();
    bool allocateCommandBuffer();
    bool createDescriptorPool();
    void updateDescriptorSets();

    void cleanupPipeline();

    uint32_t findMemoryType(uint32_t typeFilter, VkMemoryPropertyFlags properties);
};

#endif // SHADER_H
