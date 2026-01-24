#include "common.h"
#if !FLUTTER_VULKAN_SIMULATOR_STUB

#include "uniform_queue.h"

#include <iterator>
#include <any>
#include <iostream>
#include <algorithm>

#define LOGD(TAG,...) printf(TAG),printf(" "),printf(__VA_ARGS__),printf("\n");fflush(stdout);

UniformQueue::UniformQueue() {
}

UniformQueue::~UniformQueue() {
    if (vkCtx && vkCtx->device != VK_NULL_HANDLE) {
        for (auto &[name, uniform] : uniforms) {
            if (uniform.type() == typeid(UNIFORM_SAMPLER2D_t)) {
                Sampler2D &sampler = std::any_cast<UNIFORM_SAMPLER2D_t &>(uniform).val;
                sampler.destroyVulkanTexture(vkCtx->device);
            }
        }
    }
}

bool UniformQueue::addUniform(std::string name, UniformType type, void *val) {
    if (uniforms.find(name) != uniforms.end()) {
        std::cout << "Uniform \"" << name << "\"  already exists!" << std::endl;
        return false;
    }

    switch (type) {
        case UNIFORM_BOOL: {
            bool f = *(bool *) val;
            uniforms.emplace(name, UNIFORM_BOOL_t(UNIFORM_BOOL, f));
            break;
        }
        case UNIFORM_INT: {
            int f = *(int *) val;
            uniforms.emplace(name, UNIFORM_INT_t(UNIFORM_INT, f));
            break;
        }
        case UNIFORM_FLOAT: {
            float f = *(float *) val;
            uniforms.emplace(name, UNIFORM_FLOAT_t(UNIFORM_FLOAT, f));
            break;
        }
        case UNIFORM_VEC2: {
            vec2 f = *(vec2 *) val;
            uniforms.emplace(name, UNIFORM_VEC2_t(UNIFORM_VEC2, f));
            break;
        }
        case UNIFORM_VEC3: {
            vec3 f = *(vec3 *) val;
            uniforms.emplace(name, UNIFORM_VEC3_t(UNIFORM_VEC3, f));
            break;
        }
        case UNIFORM_VEC4: {
            vec4 f = *(vec4 *) val;
            uniforms.emplace(name, UNIFORM_VEC4_t(UNIFORM_VEC4, f));
            break;
        }
        case UNIFORM_MAT2: {
            mat2 f = *(mat2 *) val;
            uniforms.emplace(name, UNIFORM_MAT2_t(UNIFORM_MAT2, f));
            break;
        }
        case UNIFORM_MAT3: {
            mat3 f = *(mat3 *) val;
            uniforms.emplace(name, UNIFORM_MAT3_t(UNIFORM_MAT3, f));
            break;
        }
        case UNIFORM_MAT4: {
            mat4 f = *(mat4 *) val;
            uniforms.emplace(name, UNIFORM_MAT4_t(UNIFORM_MAT4, f));
            break;
        }
        case UNIFORM_SAMPLER2D: {
            const Sampler2D f = *(Sampler2D *)(val);
            uniforms.emplace(name, UNIFORM_SAMPLER2D_t(UNIFORM_SAMPLER2D, f));
            break;
        }
    }
    return true;
}

bool UniformQueue::removeUniform(const std::string &name)
{
    if (uniforms.find(name) == uniforms.end()) {
        std::cout << "Uniform \"" << name << "\"  doesn't exist!" << std::endl;
        return false;
    }

    if (uniforms[name].type() == typeid(UNIFORM_SAMPLER2D_t)) {
        Sampler2D &f = std::any_cast<UNIFORM_SAMPLER2D_t &>(uniforms[name]).val;
        if (vkCtx && vkCtx->device != VK_NULL_HANDLE)
            f.destroyVulkanTexture(vkCtx->device);
    }
    uniforms.erase(uniforms.find(name));
    return true;
}

bool UniformQueue::setUniformValue(const std::string &name, void *val) {
    if (uniforms.find(name) == uniforms.end()) {
        std::cout << "Uniform \"" << name << "\"  not found!" << std::endl;
        return false;
    }

    const std::type_info &t = uniforms[name].type();

    if (t == typeid(UNIFORM_BOOL_t)) {
        std::any_cast<UNIFORM_BOOL_t &>(uniforms[name]).val = *(bool *) val;
    } else if (t == typeid(UNIFORM_INT_t)) {
        std::any_cast<UNIFORM_INT_t &>(uniforms[name]).val = *(int *) val;
    } else if (t == typeid(UNIFORM_FLOAT_t)) {
        std::any_cast<UNIFORM_FLOAT_t &>(uniforms[name]).val = *(float *) val;
    } else if (t == typeid(UNIFORM_VEC2_t)) {
        std::any_cast<UNIFORM_VEC2_t &>(uniforms[name]).val = *(vec2 *) val;
    } else if (t == typeid(UNIFORM_VEC3_t)) {
        std::any_cast<UNIFORM_VEC3_t &>(uniforms[name]).val = *(vec3 *) val;
    } else if (t == typeid(UNIFORM_VEC4_t)) {
        std::any_cast<UNIFORM_VEC4_t &>(uniforms[name]).val = *(vec4 *) val;
    } else if (t == typeid(UNIFORM_MAT2_t)) {
        std::any_cast<UNIFORM_MAT2_t &>(uniforms[name]).val = *(mat2 *) val;
    } else if (t == typeid(UNIFORM_MAT3_t)) {
        std::any_cast<UNIFORM_MAT3_t &>(uniforms[name]).val = *(mat3 *) val;
    } else if (t == typeid(UNIFORM_MAT4_t)) {
        std::any_cast<UNIFORM_MAT4_t &>(uniforms[name]).val = *(mat4 *) val;
    } else if (t == typeid(UNIFORM_SAMPLER2D_t)) {
        Sampler2D &f = std::any_cast<UNIFORM_SAMPLER2D_t &>(uniforms[name]).val;
        f.add_RGBA32(f.width, f.height, (unsigned char *) val);
        return true;
    } else {
        return false;
    }

    return true;
}

PushConstants UniformQueue::getPushConstants() {
    PushConstants pc{};
    pc.iMouse = {0, 0, 0, 0};
    pc.iResolution = {0, 0, 0};
    pc.iTime = 0;

    auto it = uniforms.find("iMouse");
    if (it != uniforms.end() && it->second.type() == typeid(UNIFORM_VEC4_t)) {
        pc.iMouse = std::any_cast<UNIFORM_VEC4_t &>(it->second).val;
    }
    it = uniforms.find("iResolution");
    if (it != uniforms.end() && it->second.type() == typeid(UNIFORM_VEC3_t)) {
        pc.iResolution = std::any_cast<UNIFORM_VEC3_t &>(it->second).val;
    }
    it = uniforms.find("iTime");
    if (it != uniforms.end() && it->second.type() == typeid(UNIFORM_FLOAT_t)) {
        pc.iTime = std::any_cast<UNIFORM_FLOAT_t &>(it->second).val;
    }

    return pc;
}

void UniformQueue::setAllSampler2D()
{
    if (!vkCtx || vkCtx->device == VK_NULL_HANDLE) return;

    int n = 0;
    for (auto &[name, uniform] : uniforms) {
        if (uniform.type() == typeid(UNIFORM_SAMPLER2D_t)) {
            Sampler2D &sampler = std::any_cast<UNIFORM_SAMPLER2D_t &>(uniform).val;
            if (!sampler.data.empty()) {
                if (sampler.nTexture == -1)
                    sampler.nTexture = n;
                sampler.createVulkanTexture(vkCtx->device, vkCtx->physicalDevice,
                                            vkCtx->commandPool, vkCtx->graphicsQueue);
                n++;
            } else if (sampler.nTexture >= 0) {
                n = sampler.nTexture + 1;
            }
        }
    }
}

bool UniformQueue::replaceSampler2D(const std::string &name, int w, int h, unsigned char *rawData)
{
    if (uniforms.find(name) == uniforms.end()) return false;

    if (uniforms[name].type() == typeid(UNIFORM_SAMPLER2D_t)) {
        Sampler2D &sampler = std::any_cast<UNIFORM_SAMPLER2D_t &>(uniforms[name]).val;
        sampler.replaceTexture(w, h, rawData);
        return true;
    }
    return false;
}

Sampler2D *UniformQueue::getSampler2D(const std::string &name)
{
    if (uniforms.find(name) == uniforms.end()) return nullptr;
    if (uniforms[name].type() == typeid(UNIFORM_SAMPLER2D_t)) {
        return &std::any_cast<UNIFORM_SAMPLER2D_t &>(uniforms[name]).val;
    }
    return nullptr;
}

std::vector<std::pair<int, Sampler2D*>> UniformQueue::getAllSampler2DTextures()
{
    std::vector<std::pair<int, Sampler2D*>> result;
    for (auto &[name, uniform] : uniforms) {
        if (uniform.type() == typeid(UNIFORM_SAMPLER2D_t)) {
            Sampler2D &sampler = std::any_cast<UNIFORM_SAMPLER2D_t &>(uniform).val;
            if (sampler.nTexture >= 0 && sampler.imageView != VK_NULL_HANDLE) {
                result.push_back({sampler.nTexture, &sampler});
            }
        }
    }
    return result;
}

#endif // !FLUTTER_VULKAN_SIMULATOR_STUB
