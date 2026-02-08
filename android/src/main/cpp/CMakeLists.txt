cmake_minimum_required(VERSION 3.10)

project("flutter_vulkan_plugin" LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(SRC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../../src")

add_library(flutter_vulkan_plugin SHARED
    flutter_vulkan_jni.cpp
    ${SRC_DIR}/ffi.cpp
    ${SRC_DIR}/renderer.cpp
    ${SRC_DIR}/shader.cpp
    ${SRC_DIR}/uniform_queue.cpp
    ${SRC_DIR}/sampler2d.cpp
    ${SRC_DIR}/vulkan_context.cpp
)

target_include_directories(flutter_vulkan_plugin PRIVATE
    ${SRC_DIR}
)

# Vulkan
find_package(Vulkan REQUIRED)
target_link_libraries(flutter_vulkan_plugin PRIVATE Vulkan::Vulkan)

# Android libs
target_link_libraries(flutter_vulkan_plugin PRIVATE android log)

# shaderc — build from NDK sources via ndk-build (lazy: only when .a is missing)
set(SHADERC_SRC "${ANDROID_NDK}/sources/third_party/shaderc")
set(SHADERC_BUILD_DIR "${CMAKE_CURRENT_BINARY_DIR}/shaderc_build")
set(SHADERC_OUTPUT "${SHADERC_BUILD_DIR}/libs/c++_shared/${ANDROID_ABI}/libshaderc.a")

add_custom_command(
    OUTPUT "${SHADERC_OUTPUT}"
    COMMAND "${ANDROID_NDK}/ndk-build"
        NDK_PROJECT_PATH=${SHADERC_BUILD_DIR}
        APP_BUILD_SCRIPT=${SHADERC_SRC}/Android.mk
        APP_ABI=${ANDROID_ABI}
        APP_STL=c++_shared
        APP_PLATFORM=android-${ANDROID_NATIVE_API_LEVEL}
        libshaderc_combined
        -j4
    COMMENT "Building shaderc from NDK sources for ${ANDROID_ABI}..."
)
add_custom_target(shaderc_build DEPENDS "${SHADERC_OUTPUT}")

add_library(shaderc STATIC IMPORTED)
set_target_properties(shaderc PROPERTIES IMPORTED_LOCATION "${SHADERC_OUTPUT}")
add_dependencies(shaderc shaderc_build)

target_include_directories(flutter_vulkan_plugin PRIVATE "${SHADERC_SRC}/libshaderc/include")
target_link_libraries(flutter_vulkan_plugin PRIVATE shaderc)

# Release optimizations: -O2, hide internal symbols, strip, disable validation layers
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    target_compile_options(flutter_vulkan_plugin PRIVATE -O2 -DNDEBUG -fvisibility=hidden)
    target_link_options(flutter_vulkan_plugin PRIVATE -Wl,--gc-sections -Wl,-s)
endif()
