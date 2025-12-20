#ifndef SAMPLER2D_H
#define SAMPLER2D_H

#include <vulkan/vulkan.h>
#include <vector>
#include <cstdint>

class Sampler2D {
public:
    Sampler2D();
    ~Sampler2D() = default;

    void replaceTexture(int w, int h, unsigned char *rawData);
    void add_RGBA32(int w, int h, unsigned char *rawData);

    // Create Vulkan image/view/sampler from stored data
    void createVulkanTexture(VkDevice device, VkPhysicalDevice physDevice,
                             VkCommandPool cmdPool, VkQueue queue);
    void destroyVulkanTexture(VkDevice device);

    std::vector<unsigned char> data;
    int width = 0;
    int height = 0;
    int nTexture = -1;

    // Vulkan resources
    VkImage image = VK_NULL_HANDLE;
    VkDeviceMemory imageMemory = VK_NULL_HANDLE;
    VkImageView imageView = VK_NULL_HANDLE;
    VkSampler sampler = VK_NULL_HANDLE;

private:
    uint32_t findMemoryType(VkPhysicalDevice physDevice, uint32_t typeFilter,
                            VkMemoryPropertyFlags properties);
};

#endif // SAMPLER2D_H
