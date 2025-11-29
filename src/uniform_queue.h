#ifndef UNIFORM_QUEUE_H
#define UNIFORM_QUEUE_H

#include "sampler2d.h"
#include "vulkan_context.h"

#include <iostream>
#include <vector>
#include <string>
#include <map>
#include <any>
#include <cstring>

typedef enum {
    UNIFORM_BOOL,
    UNIFORM_INT,
    UNIFORM_FLOAT,
    UNIFORM_VEC2,
    UNIFORM_VEC3,
    UNIFORM_VEC4,
    UNIFORM_MAT2,
    UNIFORM_MAT3,
    UNIFORM_MAT4,
    UNIFORM_SAMPLER2D
} UniformType;

// Simple vec/mat types to avoid GLM dependency
struct vec2 { float x, y; };
struct vec3 { float x, y, z; };
struct vec4 { float x, y, z, w; };
struct mat2 { float data[4]; };
struct mat3 { float data[9]; };
struct mat4 { float data[16]; };

// Push constants for ShaderToy built-in uniforms
struct PushConstants {
    vec4 iMouse;       // 16 bytes
    vec3 iResolution;  // 12 bytes
    float iTime;       // 4 bytes
    // Total: 32 bytes (well within 128-byte minimum)
};

class UniformQueue {
public:
    UniformQueue();
    ~UniformQueue();

    void setVulkanContext(VulkanContext *ctx) { vkCtx = ctx; }

    bool addUniform(std::string name, UniformType type, void *val);
    bool removeUniform(const std::string &name);
    bool setUniformValue(const std::string &name, void *val);

    // Fill push constants struct from stored uniforms
    PushConstants getPushConstants();

    // Vulkan texture management
    void setAllSampler2D();
    bool replaceSampler2D(const std::string &name, int w, int h, unsigned char *rawData);
    Sampler2D *getSampler2D(const std::string &name);

    // Get all sampler2D textures for descriptor set binding
    std::vector<std::pair<int, Sampler2D*>> getAllSampler2DTextures();

    template<typename T>
    struct UniformStruct {
        UniformType type;
        T val;
        UniformStruct(UniformType type, const T &data)
                : type(type), val(data) {};
    };

    typedef UniformStruct<bool> UNIFORM_BOOL_t;
    typedef UniformStruct<int> UNIFORM_INT_t;
    typedef UniformStruct<float> UNIFORM_FLOAT_t;
    typedef UniformStruct<vec2> UNIFORM_VEC2_t;
    typedef UniformStruct<vec3> UNIFORM_VEC3_t;
    typedef UniformStruct<vec4> UNIFORM_VEC4_t;
    typedef UniformStruct<mat2> UNIFORM_MAT2_t;
    typedef UniformStruct<mat3> UNIFORM_MAT3_t;
    typedef UniformStruct<mat4> UNIFORM_MAT4_t;
    typedef UniformStruct<Sampler2D> UNIFORM_SAMPLER2D_t;

    std::map<std::string, std::any> uniforms;

private:
    VulkanContext *vkCtx = nullptr;
};

#endif // UNIFORM_QUEUE_H
