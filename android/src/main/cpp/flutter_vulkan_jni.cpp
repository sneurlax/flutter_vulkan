#include <jni.h>
#include <cstdint>
#include <cstring>

extern "C" {
    void createRenderer(uint8_t *buffer, int width, int height);
    void deleteRenderer();
    void *getRenderer();
    void stopThread();
    void setFrameCallback(void (*callback)(void*), void* user_data);
}

static uint8_t *pixelBuffer = nullptr;
static int bufWidth = 0, bufHeight = 0;

extern "C" JNIEXPORT void JNICALL
Java_com_example_flutter_1vulkan_FlutterVulkanPlugin_nativeCreateSurface(
        JNIEnv *env, jobject thiz, jobject surface, jint width, jint height) {
    bufWidth = width;
    bufHeight = height;
    delete[] pixelBuffer;
    pixelBuffer = new uint8_t[width * height * 4]();
    createRenderer(pixelBuffer, width, height);
}

extern "C" JNIEXPORT void JNICALL
Java_com_example_flutter_1vulkan_FlutterVulkanPlugin_nativeDestroySurface(
        JNIEnv *env, jobject thiz) {
    if (getRenderer() != nullptr) stopThread();
    deleteRenderer();
    delete[] pixelBuffer;
    pixelBuffer = nullptr;
}
