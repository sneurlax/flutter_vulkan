#include <jni.h>
#include <android/native_window.h>
#include <android/native_window_jni.h>

#include "../../../../src/common.h"
#include "../../../../src/ffi.h"

#define LOG_TAG_JNI "FLUTTER_VULKAN_JNI"

static ANativeWindow *nativeWindow = nullptr;

extern "C" JNIEXPORT void JNICALL
Java_com_example_flutter_1vulkan_FlutterVulkanPlugin_nativeCreateSurface(
        JNIEnv *env, jobject thiz, jobject surface, jint width, jint height) {
    nativeWindow = ANativeWindow_fromSurface(env, surface);
    if (nativeWindow == nullptr) {
        LOGD(LOG_TAG_JNI, "Failed to get ANativeWindow from Surface");
        return;
    }

    ANativeWindow_setBuffersGeometry(nativeWindow, width, height, WINDOW_FORMAT_RGBA_8888);

    ctx_f.window = nativeWindow;
    ctx_f.width = width;
    ctx_f.height = height;

    createRenderer(&ctx_f);
    LOGD(LOG_TAG_JNI, "Surface created: %dx%d", width, height);
}

extern "C" JNIEXPORT void JNICALL
Java_com_example_flutter_1vulkan_FlutterVulkanPlugin_nativeDestroySurface(
        JNIEnv *env, jobject thiz) {
    if (getRenderer() != nullptr) {
        stopThread();
    }
    if (nativeWindow != nullptr) {
        ANativeWindow_release(nativeWindow);
        nativeWindow = nullptr;
    }
    ctx_f.window = nullptr;
    LOGD(LOG_TAG_JNI, "Surface destroyed");
}
