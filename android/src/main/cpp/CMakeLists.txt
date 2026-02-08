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

# shaderc
set(PLUGIN_SHADERC "${CMAKE_CURRENT_SOURCE_DIR}/../../../../third_party/shaderc")
set(SHADERC_LIB "${PLUGIN_SHADERC}/lib/${ANDROID_ABI}/libshaderc.a")

if(NOT EXISTS "${SHADERC_LIB}")
    message(FATAL_ERROR "Could not find shaderc at ${SHADERC_LIB}. Build it with ndk-build from ${ANDROID_NDK}/sources/third_party/shaderc/.")
endif()

add_library(shaderc STATIC IMPORTED)
set_target_properties(shaderc PROPERTIES IMPORTED_LOCATION "${SHADERC_LIB}")
target_include_directories(flutter_vulkan_plugin PRIVATE "${PLUGIN_SHADERC}/include")
target_link_libraries(flutter_vulkan_plugin PRIVATE shaderc)
