#include <jni.h>
#include <cstdint>
#include <cstring>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <android/log.h>

#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, "FlutterVulkanJNI", __VA_ARGS__)

extern "C" {
    void createRenderer(uint8_t *buffer, int width, int height);
    void deleteRenderer();
    void *getRenderer();
    void stopThread();
    void setFrameCallback(void (*callback)(void*), void* user_data);
}

static uint8_t *pixelBuffer = nullptr;
static int bufWidth = 0, bufHeight = 0;
static ANativeWindow *nativeWindow = nullptr;

static void onFrameReady(void *userData) {
    auto *window = static_cast<ANativeWindow *>(userData);
    if (!window || !pixelBuffer) return;

    ANativeWindow_Buffer nBuf;
    if (ANativeWindow_lock(window, &nBuf, nullptr) == 0) {
        auto *src = pixelBuffer;
        auto *dst = static_cast<uint8_t *>(nBuf.bits);
        int srcStride = bufWidth * 4;
        int dstStride = nBuf.stride * 4;
        for (int y = 0; y < bufHeight; y++) {
            memcpy(dst + y * dstStride, src + y * srcStride, srcStride);
        }
        ANativeWindow_unlockAndPost(window);
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_example_flutter_1vulkan_FlutterVulkanPlugin_nativeCreateSurface(
        JNIEnv *env, jobject thiz, jobject surface, jint width, jint height) {
    bufWidth = width;
    bufHeight = height;

    // Release previous native window
    if (nativeWindow) {
        ANativeWindow_release(nativeWindow);
        nativeWindow = nullptr;
    }

    delete[] pixelBuffer;
    pixelBuffer = new uint8_t[width * height * 4]();

    // Get ANativeWindow from the Surface
    nativeWindow = ANativeWindow_fromSurface(env, surface);
    ANativeWindow_setBuffersGeometry(nativeWindow, width, height, AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM);

    createRenderer(pixelBuffer, width, height);

    // Set the frame callback so the render loop pushes pixels to the surface
    setFrameCallback(onFrameReady, nativeWindow);
}

extern "C" JNIEXPORT void JNICALL
Java_com_example_flutter_1vulkan_FlutterVulkanPlugin_nativeDestroySurface(
        JNIEnv *env, jobject thiz) {
    if (getRenderer() != nullptr) stopThread();
    deleteRenderer();
    delete[] pixelBuffer;
    pixelBuffer = nullptr;
    if (nativeWindow) {
        ANativeWindow_release(nativeWindow);
        nativeWindow = nullptr;
    }
}
