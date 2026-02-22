#include "shader.h"

#include <ctime>
#include <cstring>
#include <shaderc/shaderc.hpp>

#define LOG_TAG_SHADER "NATIVE SHADER"

#ifdef _IS_MACOS_
// macOS CVPixelBuffer uses BGRA format
#define FLUTTER_VK_COLOR_FORMAT VK_FORMAT_B8G8R8A8_UNORM
#else
#define FLUTTER_VK_COLOR_FORMAT VK_FORMAT_R8G8B8A8_UNORM
#endif

Shader::Shader(VulkanPluginContext *pluginCtx, VulkanContext *vkCtx)
        : self(pluginCtx),
          vkCtx(vkCtx),
          _isContinuous(true),
          pipelineValid(false),
          uniformsList(UniformQueue()) {
    uniformsList.setVulkanContext(vkCtx);
}

Shader::~Shader() {
    if (vkCtx && vkCtx->device != VK_NULL_HANDLE) {
        vkDeviceWaitIdle(vkCtx->device);
    }
    cleanupPipeline();
}

void Shader::setIsContinuous(bool isContinuous) {
    _isContinuous = isContinuous;
}

void Shader::addShaderToyUniforms() {
    vec4 iMouse = {0.0f, 0.0f, 0.0f, 0.0f};
    vec3 iResolution = {(float)width, (float)height, 0.0f};
    float time = 0.0f;
    uniformsList.addUniform("iMouse", UNIFORM_VEC4, (void *)(&iMouse));
    uniformsList.addUniform("iResolution", UNIFORM_VEC3, (void *)(&iResolution));
    uniformsList.addUniform("iTime", UNIFORM_FLOAT, (void *)(&time));

    // Add opaque black 4x4 textures for iChannel[0-3]
    std::vector<unsigned char> rawData(4 * 4 * 4, 0);
    for (int i = 3; i < 4 * 4 * 4; i += 4) rawData[i] = 255;
    Sampler2D sampler;
    sampler.add_RGBA32(4, 4, rawData.data());
    uniformsList.addUniform("iChannel0", UNIFORM_SAMPLER2D, (void *)(&sampler));
    uniformsList.addUniform("iChannel1", UNIFORM_SAMPLER2D, (void *)(&sampler));
    uniformsList.addUniform("iChannel2", UNIFORM_SAMPLER2D, (void *)(&sampler));
    uniformsList.addUniform("iChannel3", UNIFORM_SAMPLER2D, (void *)(&sampler));
}

void Shader::setShadersSize(int w, int h) {
    width = w;
    height = h;
}

void Shader::setShadersText(std::string vs, std::string fs) {
    vertexSource = vs;
    fragmentSource = fs;
}

std::vector<uint32_t> Shader::compileGLSLToSPIRV(const std::string &source, bool isVertex) {
    shaderc::Compiler compiler;
    shaderc::CompileOptions options;
    options.SetTargetEnvironment(shaderc_target_env_vulkan, shaderc_env_version_vulkan_1_0);
    options.SetOptimizationLevel(shaderc_optimization_level_performance);

    auto kind = isVertex ? shaderc_vertex_shader : shaderc_fragment_shader;
    auto result = compiler.CompileGlslToSpv(source, kind, isVertex ? "vertex.glsl" : "fragment.glsl", options);

    if (result.GetCompilationStatus() != shaderc_compilation_status_success) {
        std::string type = isVertex ? "VERTEX" : "FRAGMENT";
        compileError = type + " shader compile error:\n" + result.GetErrorMessage();
        LOGD(LOG_TAG_SHADER, "%s", compileError.c_str());
        return {};
    }

    return {result.cbegin(), result.cend()};
}

uint32_t Shader::findMemoryType(uint32_t typeFilter, VkMemoryPropertyFlags properties) {
    VkPhysicalDeviceMemoryProperties memProperties;
    vkGetPhysicalDeviceMemoryProperties(vkCtx->physicalDevice, &memProperties);

    for (uint32_t i = 0; i < memProperties.memoryTypeCount; i++) {
        if ((typeFilter & (1 << i)) &&
            (memProperties.memoryTypes[i].propertyFlags & properties) == properties) {
            return i;
        }
    }
    return 0;
}

bool Shader::createRenderPass() {
    VkAttachmentDescription colorAttachment{};
    colorAttachment.format = FLUTTER_VK_COLOR_FORMAT;
    colorAttachment.samples = VK_SAMPLE_COUNT_1_BIT;
    colorAttachment.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
    colorAttachment.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
    colorAttachment.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
    colorAttachment.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
    colorAttachment.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    colorAttachment.finalLayout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;

    VkAttachmentReference colorAttachmentRef{};
    colorAttachmentRef.attachment = 0;
    colorAttachmentRef.layout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

    VkSubpassDescription subpass{};
    subpass.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS;
    subpass.colorAttachmentCount = 1;
    subpass.pColorAttachments = &colorAttachmentRef;

    VkRenderPassCreateInfo renderPassInfo{};
    renderPassInfo.sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
    renderPassInfo.attachmentCount = 1;
    renderPassInfo.pAttachments = &colorAttachment;
    renderPassInfo.subpassCount = 1;
    renderPassInfo.pSubpasses = &subpass;

    return vkCreateRenderPass(vkCtx->device, &renderPassInfo, nullptr, &renderPass) == VK_SUCCESS;
}

bool Shader::createDescriptorSetLayout() {
    VkDescriptorSetLayoutBinding bindings[4]{};
    for (int i = 0; i < 4; i++) {
        bindings[i].binding = i;
        bindings[i].descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
        bindings[i].descriptorCount = 1;
        bindings[i].stageFlags = VK_SHADER_STAGE_FRAGMENT_BIT;
    }

    VkDescriptorSetLayoutCreateInfo layoutInfo{};
    layoutInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO;
    layoutInfo.bindingCount = 4;
    layoutInfo.pBindings = bindings;

    return vkCreateDescriptorSetLayout(vkCtx->device, &layoutInfo, nullptr, &descriptorSetLayout) == VK_SUCCESS;
}

bool Shader::createPipeline(const std::vector<uint32_t> &vertSpirv,
                             const std::vector<uint32_t> &fragSpirv) {
    // Create shader modules
    VkShaderModuleCreateInfo vertModuleInfo{};
    vertModuleInfo.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    vertModuleInfo.codeSize = vertSpirv.size() * sizeof(uint32_t);
    vertModuleInfo.pCode = vertSpirv.data();

    VkShaderModule vertModule;
    if (vkCreateShaderModule(vkCtx->device, &vertModuleInfo, nullptr, &vertModule) != VK_SUCCESS) {
        LOGD(LOG_TAG_SHADER, "Failed to create vertex shader module");
        return false;
    }

    VkShaderModuleCreateInfo fragModuleInfo{};
    fragModuleInfo.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    fragModuleInfo.codeSize = fragSpirv.size() * sizeof(uint32_t);
    fragModuleInfo.pCode = fragSpirv.data();

    VkShaderModule fragModule;
    if (vkCreateShaderModule(vkCtx->device, &fragModuleInfo, nullptr, &fragModule) != VK_SUCCESS) {
        vkDestroyShaderModule(vkCtx->device, vertModule, nullptr);
        LOGD(LOG_TAG_SHADER, "Failed to create fragment shader module");
        return false;
    }

    VkPipelineShaderStageCreateInfo shaderStages[2]{};
    shaderStages[0].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    shaderStages[0].stage = VK_SHADER_STAGE_VERTEX_BIT;
    shaderStages[0].module = vertModule;
    shaderStages[0].pName = "main";
    shaderStages[1].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    shaderStages[1].stage = VK_SHADER_STAGE_FRAGMENT_BIT;
    shaderStages[1].module = fragModule;
    shaderStages[1].pName = "main";

    // Empty vertex input (full-screen triangle generated in vertex shader)
    VkPipelineVertexInputStateCreateInfo vertexInputInfo{};
    vertexInputInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;

    VkPipelineInputAssemblyStateCreateInfo inputAssembly{};
    inputAssembly.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
    inputAssembly.topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;

    VkViewport viewport{};
    viewport.x = 0.0f;
    viewport.y = 0.0f;
    viewport.width = (float)width;
    viewport.height = (float)height;
    viewport.minDepth = 0.0f;
    viewport.maxDepth = 1.0f;

    VkRect2D scissor{};
    scissor.offset = {0, 0};
    scissor.extent = {(uint32_t)width, (uint32_t)height};

    VkPipelineViewportStateCreateInfo viewportState{};
    viewportState.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
    viewportState.viewportCount = 1;
    viewportState.pViewports = &viewport;
    viewportState.scissorCount = 1;
    viewportState.pScissors = &scissor;

    VkPipelineRasterizationStateCreateInfo rasterizer{};
    rasterizer.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
    rasterizer.polygonMode = VK_POLYGON_MODE_FILL;
    rasterizer.lineWidth = 1.0f;
    rasterizer.cullMode = VK_CULL_MODE_NONE;
    rasterizer.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE;

    VkPipelineMultisampleStateCreateInfo multisampling{};
    multisampling.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
    multisampling.rasterizationSamples = VK_SAMPLE_COUNT_1_BIT;

    VkPipelineColorBlendAttachmentState colorBlendAttachment{};
    colorBlendAttachment.colorWriteMask =
        VK_COLOR_COMPONENT_R_BIT | VK_COLOR_COMPONENT_G_BIT |
        VK_COLOR_COMPONENT_B_BIT | VK_COLOR_COMPONENT_A_BIT;
    colorBlendAttachment.blendEnable = VK_FALSE;

    VkPipelineColorBlendStateCreateInfo colorBlending{};
    colorBlending.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
    colorBlending.attachmentCount = 1;
    colorBlending.pAttachments = &colorBlendAttachment;

    // Push constant range
    VkPushConstantRange pushConstantRange{};
    pushConstantRange.stageFlags = VK_SHADER_STAGE_FRAGMENT_BIT;
    pushConstantRange.offset = 0;
    pushConstantRange.size = sizeof(PushConstants);

    // Pipeline layout
    VkPipelineLayoutCreateInfo pipelineLayoutInfo{};
    pipelineLayoutInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    pipelineLayoutInfo.setLayoutCount = 1;
    pipelineLayoutInfo.pSetLayouts = &descriptorSetLayout;
    pipelineLayoutInfo.pushConstantRangeCount = 1;
    pipelineLayoutInfo.pPushConstantRanges = &pushConstantRange;

    if (vkCreatePipelineLayout(vkCtx->device, &pipelineLayoutInfo, nullptr, &pipelineLayout) != VK_SUCCESS) {
        vkDestroyShaderModule(vkCtx->device, vertModule, nullptr);
        vkDestroyShaderModule(vkCtx->device, fragModule, nullptr);
        return false;
    }

    VkGraphicsPipelineCreateInfo pipelineInfo{};
    pipelineInfo.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
    pipelineInfo.stageCount = 2;
    pipelineInfo.pStages = shaderStages;
    pipelineInfo.pVertexInputState = &vertexInputInfo;
    pipelineInfo.pInputAssemblyState = &inputAssembly;
    pipelineInfo.pViewportState = &viewportState;
    pipelineInfo.pRasterizationState = &rasterizer;
    pipelineInfo.pMultisampleState = &multisampling;
    pipelineInfo.pColorBlendState = &colorBlending;
    pipelineInfo.layout = pipelineLayout;
    pipelineInfo.renderPass = renderPass;
    pipelineInfo.subpass = 0;

    VkResult result = vkCreateGraphicsPipelines(vkCtx->device, VK_NULL_HANDLE, 1,
                                                 &pipelineInfo, nullptr, &graphicsPipeline);

    vkDestroyShaderModule(vkCtx->device, vertModule, nullptr);
    vkDestroyShaderModule(vkCtx->device, fragModule, nullptr);

    if (result != VK_SUCCESS) {
        LOGD(LOG_TAG_SHADER, "Failed to create graphics pipeline: %d", result);
        return false;
    }
    return true;
}

bool Shader::createOffscreenResources() {
    // Create color image
    VkImageCreateInfo imageInfo{};
    imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    imageInfo.imageType = VK_IMAGE_TYPE_2D;
    imageInfo.format = FLUTTER_VK_COLOR_FORMAT;
    imageInfo.extent = {(uint32_t)width, (uint32_t)height, 1};
    imageInfo.mipLevels = 1;
    imageInfo.arrayLayers = 1;
    imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    imageInfo.tiling = VK_IMAGE_TILING_OPTIMAL;
    imageInfo.usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;

    if (vkCreateImage(vkCtx->device, &imageInfo, nullptr, &colorImage) != VK_SUCCESS)
        return false;

    VkMemoryRequirements memReqs;
    vkGetImageMemoryRequirements(vkCtx->device, colorImage, &memReqs);

    VkMemoryAllocateInfo allocInfo{};
    allocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    allocInfo.allocationSize = memReqs.size;
    allocInfo.memoryTypeIndex = findMemoryType(memReqs.memoryTypeBits,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

    if (vkAllocateMemory(vkCtx->device, &allocInfo, nullptr, &colorImageMemory) != VK_SUCCESS)
        return false;
    vkBindImageMemory(vkCtx->device, colorImage, colorImageMemory, 0);

    // Create image view
    VkImageViewCreateInfo viewInfo{};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.image = colorImage;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = FLUTTER_VK_COLOR_FORMAT;
    viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    viewInfo.subresourceRange.baseMipLevel = 0;
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.baseArrayLayer = 0;
    viewInfo.subresourceRange.layerCount = 1;

    if (vkCreateImageView(vkCtx->device, &viewInfo, nullptr, &colorImageView) != VK_SUCCESS)
        return false;

    // Create framebuffer
    VkFramebufferCreateInfo fbInfo{};
    fbInfo.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
    fbInfo.renderPass = renderPass;
    fbInfo.attachmentCount = 1;
    fbInfo.pAttachments = &colorImageView;
    fbInfo.width = width;
    fbInfo.height = height;
    fbInfo.layers = 1;

    return vkCreateFramebuffer(vkCtx->device, &fbInfo, nullptr, &framebuffer) == VK_SUCCESS;
}

bool Shader::createStagingBuffer() {
    VkDeviceSize bufferSize = width * height * 4;

    VkBufferCreateInfo bufferInfo{};
    bufferInfo.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    bufferInfo.size = bufferSize;
    bufferInfo.usage = VK_BUFFER_USAGE_TRANSFER_DST_BIT;
    bufferInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;

    if (vkCreateBuffer(vkCtx->device, &bufferInfo, nullptr, &stagingBuffer) != VK_SUCCESS)
        return false;

    VkMemoryRequirements memReqs;
    vkGetBufferMemoryRequirements(vkCtx->device, stagingBuffer, &memReqs);

    VkMemoryAllocateInfo allocInfo{};
    allocInfo.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
    allocInfo.allocationSize = memReqs.size;
    allocInfo.memoryTypeIndex = findMemoryType(memReqs.memoryTypeBits,
        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT);

    if (vkAllocateMemory(vkCtx->device, &allocInfo, nullptr, &stagingBufferMemory) != VK_SUCCESS)
        return false;

    vkBindBufferMemory(vkCtx->device, stagingBuffer, stagingBufferMemory, 0);
    vkMapMemory(vkCtx->device, stagingBufferMemory, 0, bufferSize, 0, &stagingMapped);

    return true;
}

bool Shader::allocateCommandBuffer() {
    VkCommandBufferAllocateInfo allocInfo{};
    allocInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    allocInfo.commandPool = vkCtx->commandPool;
    allocInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    allocInfo.commandBufferCount = 1;

    return vkAllocateCommandBuffers(vkCtx->device, &allocInfo, &commandBuffer) == VK_SUCCESS;
}

bool Shader::createDescriptorPool() {
    VkDescriptorPoolSize poolSize{};
    poolSize.type = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
    poolSize.descriptorCount = 4;

    VkDescriptorPoolCreateInfo poolInfo{};
    poolInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO;
    poolInfo.poolSizeCount = 1;
    poolInfo.pPoolSizes = &poolSize;
    poolInfo.maxSets = 1;

    if (vkCreateDescriptorPool(vkCtx->device, &poolInfo, nullptr, &descriptorPool) != VK_SUCCESS)
        return false;

    VkDescriptorSetAllocateInfo setAllocInfo{};
    setAllocInfo.sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO;
    setAllocInfo.descriptorPool = descriptorPool;
    setAllocInfo.descriptorSetCount = 1;
    setAllocInfo.pSetLayouts = &descriptorSetLayout;

    return vkAllocateDescriptorSets(vkCtx->device, &setAllocInfo, &descriptorSet) == VK_SUCCESS;
}

void Shader::updateDescriptorSets() {
    auto textures = uniformsList.getAllSampler2DTextures();

    // We need 4 combined image samplers for iChannel0-3
    // For bindings without a valid texture, use the first available or skip
    VkDescriptorImageInfo imageInfos[4]{};
    VkWriteDescriptorSet writes[4]{};
    int writeCount = 0;

    for (auto &[binding, sampler] : textures) {
        if (binding >= 0 && binding < 4) {
            imageInfos[binding].imageLayout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
            imageInfos[binding].imageView = sampler->imageView;
            imageInfos[binding].sampler = sampler->sampler;

            writes[writeCount].sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
            writes[writeCount].dstSet = descriptorSet;
            writes[writeCount].dstBinding = binding;
            writes[writeCount].dstArrayElement = 0;
            writes[writeCount].descriptorType = VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER;
            writes[writeCount].descriptorCount = 1;
            writes[writeCount].pImageInfo = &imageInfos[binding];
            writeCount++;
        }
    }

    if (writeCount > 0) {
        vkUpdateDescriptorSets(vkCtx->device, writeCount, writes, 0, nullptr);
    }
}

void Shader::cleanupPipeline() {
    if (!vkCtx || vkCtx->device == VK_NULL_HANDLE) return;

    if (commandBuffer != VK_NULL_HANDLE) {
        vkFreeCommandBuffers(vkCtx->device, vkCtx->commandPool, 1, &commandBuffer);
        commandBuffer = VK_NULL_HANDLE;
    }
    if (stagingMapped) {
        vkUnmapMemory(vkCtx->device, stagingBufferMemory);
        stagingMapped = nullptr;
    }
    if (stagingBuffer != VK_NULL_HANDLE) {
        vkDestroyBuffer(vkCtx->device, stagingBuffer, nullptr);
        stagingBuffer = VK_NULL_HANDLE;
    }
    if (stagingBufferMemory != VK_NULL_HANDLE) {
        vkFreeMemory(vkCtx->device, stagingBufferMemory, nullptr);
        stagingBufferMemory = VK_NULL_HANDLE;
    }
    if (framebuffer != VK_NULL_HANDLE) {
        vkDestroyFramebuffer(vkCtx->device, framebuffer, nullptr);
        framebuffer = VK_NULL_HANDLE;
    }
    if (colorImageView != VK_NULL_HANDLE) {
        vkDestroyImageView(vkCtx->device, colorImageView, nullptr);
        colorImageView = VK_NULL_HANDLE;
    }
    if (colorImage != VK_NULL_HANDLE) {
        vkDestroyImage(vkCtx->device, colorImage, nullptr);
        colorImage = VK_NULL_HANDLE;
    }
    if (colorImageMemory != VK_NULL_HANDLE) {
        vkFreeMemory(vkCtx->device, colorImageMemory, nullptr);
        colorImageMemory = VK_NULL_HANDLE;
    }
    if (descriptorPool != VK_NULL_HANDLE) {
        vkDestroyDescriptorPool(vkCtx->device, descriptorPool, nullptr);
        descriptorPool = VK_NULL_HANDLE;
    }
    if (graphicsPipeline != VK_NULL_HANDLE) {
        vkDestroyPipeline(vkCtx->device, graphicsPipeline, nullptr);
        graphicsPipeline = VK_NULL_HANDLE;
    }
    if (pipelineLayout != VK_NULL_HANDLE) {
        vkDestroyPipelineLayout(vkCtx->device, pipelineLayout, nullptr);
        pipelineLayout = VK_NULL_HANDLE;
    }
    if (descriptorSetLayout != VK_NULL_HANDLE) {
        vkDestroyDescriptorSetLayout(vkCtx->device, descriptorSetLayout, nullptr);
        descriptorSetLayout = VK_NULL_HANDLE;
    }
    if (renderPass != VK_NULL_HANDLE) {
        vkDestroyRenderPass(vkCtx->device, renderPass, nullptr);
        renderPass = VK_NULL_HANDLE;
    }

    pipelineValid = false;
}

std::string Shader::initShader() {
    compileError = "";

    cleanupPipeline();

    // Compile shaders to SPIR-V
    auto vertSpirv = compileGLSLToSPIRV(vertexSource, true);
    if (vertSpirv.empty()) return compileError;

    auto fragSpirv = compileGLSLToSPIRV(fragmentSource, false);
    if (fragSpirv.empty()) return compileError;

    // Create Vulkan pipeline
    if (!createRenderPass()) {
        compileError = "Failed to create render pass";
        return compileError;
    }
    if (!createDescriptorSetLayout()) {
        compileError = "Failed to create descriptor set layout";
        return compileError;
    }
    if (!createPipeline(vertSpirv, fragSpirv)) {
        if (compileError.empty()) compileError = "Failed to create graphics pipeline";
        return compileError;
    }
    if (!createOffscreenResources()) {
        compileError = "Failed to create offscreen resources";
        return compileError;
    }
    if (!createStagingBuffer()) {
        compileError = "Failed to create staging buffer";
        return compileError;
    }
    if (!allocateCommandBuffer()) {
        compileError = "Failed to allocate command buffer";
        return compileError;
    }
    if (!createDescriptorPool()) {
        compileError = "Failed to create descriptor pool";
        return compileError;
    }

    // Initialize sampler2D textures
    uniformsList.setAllSampler2D();
    updateDescriptorSets();

    startTime = (float)clock() / (float)CLOCKS_PER_SEC;
    pipelineValid = true;

    LOGD(LOG_TAG_SHADER, "Vulkan pipeline created successfully (%dx%d)", width, height);
    return compileError;
}

void Shader::refreshTextures() {
    uniformsList.setAllSampler2D();
    updateDescriptorSets();
}

std::string Shader::initShaderToy() {
    // Full-screen triangle vertex shader (no vertex buffer needed)
    vertexSource =
        "#version 450\n"
        "void main() {\n"
        "    vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);\n"
        "    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);\n"
        "}\n";

    // Wrap ShaderToy fragment source with Vulkan GLSL 450 header
    std::string header =
        "#version 450\n"
        "layout(push_constant) uniform PushConstants {\n"
        "    vec4 iMouse;\n"
        "    vec3 iResolution;\n"
        "    float iTime;\n"
        "} pc;\n"
        "#define iMouse pc.iMouse\n"
        "#define iResolution pc.iResolution\n"
        "#define iTime pc.iTime\n"
        "layout(set=0, binding=0) uniform sampler2D iChannel0;\n"
        "layout(set=0, binding=1) uniform sampler2D iChannel1;\n"
        "layout(set=0, binding=2) uniform sampler2D iChannel2;\n"
        "layout(set=0, binding=3) uniform sampler2D iChannel3;\n"
        "layout(location=0) out vec4 fragColor;\n";

    std::string footer =
        "\nvoid main() {\n"
        "    mainImage(fragColor, vec2(gl_FragCoord.x, iResolution.y - gl_FragCoord.y));\n"
        "    fragColor.a = 1.0;\n"
        "}\n";

    fragmentSource = header + fragmentSource + footer;

    addShaderToyUniforms();
    return initShader();
}

void Shader::drawFrame() {
    if (!pipelineValid) return;
    std::lock_guard<std::mutex> lock_guard(mutex_);

    float time = (float)clock() / (float)CLOCKS_PER_SEC - startTime;
    uniformsList.setUniformValue("iTime", (void *)(&time));

    // Get push constants
    PushConstants pc = uniformsList.getPushConstants();

    // Reset and begin command buffer
    vkResetCommandBuffer(commandBuffer, 0);

    VkCommandBufferBeginInfo beginInfo{};
    beginInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    beginInfo.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;
    vkBeginCommandBuffer(commandBuffer, &beginInfo);

    // Begin render pass
    VkClearValue clearColor = {{{0.0f, 0.0f, 0.0f, 1.0f}}};
    VkRenderPassBeginInfo renderPassInfo{};
    renderPassInfo.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
    renderPassInfo.renderPass = renderPass;
    renderPassInfo.framebuffer = framebuffer;
    renderPassInfo.renderArea.offset = {0, 0};
    renderPassInfo.renderArea.extent = {(uint32_t)width, (uint32_t)height};
    renderPassInfo.clearValueCount = 1;
    renderPassInfo.pClearValues = &clearColor;

    vkCmdBeginRenderPass(commandBuffer, &renderPassInfo, VK_SUBPASS_CONTENTS_INLINE);

    vkCmdBindPipeline(commandBuffer, VK_PIPELINE_BIND_POINT_GRAPHICS, graphicsPipeline);

    // Push constants
    vkCmdPushConstants(commandBuffer, pipelineLayout,
                       VK_SHADER_STAGE_FRAGMENT_BIT, 0,
                       sizeof(PushConstants), &pc);

    // Bind descriptor sets (iChannel textures)
    vkCmdBindDescriptorSets(commandBuffer, VK_PIPELINE_BIND_POINT_GRAPHICS,
                            pipelineLayout, 0, 1, &descriptorSet, 0, nullptr);

    // Draw full-screen triangle (3 vertices, no vertex buffer)
    vkCmdDraw(commandBuffer, 3, 1, 0, 0);

    vkCmdEndRenderPass(commandBuffer);

    // The render pass transitions the image to TRANSFER_SRC_OPTIMAL via finalLayout.
    // Copy color image to staging buffer
    VkBufferImageCopy region{};
    region.bufferOffset = 0;
    region.bufferRowLength = 0;
    region.bufferImageHeight = 0;
    region.imageSubresource.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    region.imageSubresource.mipLevel = 0;
    region.imageSubresource.baseArrayLayer = 0;
    region.imageSubresource.layerCount = 1;
    region.imageOffset = {0, 0, 0};
    region.imageExtent = {(uint32_t)width, (uint32_t)height, 1};

    vkCmdCopyImageToBuffer(commandBuffer, colorImage,
                           VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                           stagingBuffer, 1, &region);

    vkEndCommandBuffer(commandBuffer);

    // Submit and wait
    VkSubmitInfo submitInfo{};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &commandBuffer;

    vkQueueSubmit(vkCtx->graphicsQueue, 1, &submitInfo, VK_NULL_HANDLE);
    vkQueueWaitIdle(vkCtx->graphicsQueue);

    // Copy pixels to Flutter texture buffer
#ifdef _IS_ANDROID_
    ANativeWindow_Buffer nBuf;
    if (ANativeWindow_lock(self->window, &nBuf, nullptr) == 0) {
        auto *src = static_cast<uint8_t *>(stagingMapped);
        auto *dst = static_cast<uint8_t *>(nBuf.bits);
        int srcStride = width * 4;
        int dstStride = nBuf.stride * 4;
        for (int y = 0; y < height; y++) {
            memcpy(dst + y * dstStride, src + y * srcStride, srcStride);
        }
        ANativeWindow_unlockAndPost(self->window);
    }
#elif defined(_IS_LINUX_)
    memcpy(self->myTexture->buffer, stagingMapped, width * height * 4);
    fl_texture_registrar_mark_texture_frame_available(
        self->texture_registrar, self->texture);
#elif defined(_IS_MACOS_)
    {
        auto *src = static_cast<uint8_t *>(stagingMapped);
        auto *dst = self->buffer;
        int srcStride = width * 4;
        int dstStride = self->bytesPerRow;
        if (srcStride == dstStride) {
            memcpy(dst, src, width * height * 4);
        } else {
            for (int y = 0; y < height; y++) {
                memcpy(dst + y * dstStride, src + y * srcStride, srcStride);
            }
        }
    }
    if (self->markFrameAvailable) {
        self->markFrameAvailable(self->registryRef);
    }
#endif
}
