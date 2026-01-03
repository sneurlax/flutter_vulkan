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

# shaderc - try NDK bundled version first, then plugin's third_party
set(NDK_SHADERC "${ANDROID_NDK}/sources/third_party/shaderc")
if(EXISTS "${NDK_SHADERC}")
    target_include_directories(flutter_vulkan_plugin PRIVATE "${NDK_SHADERC}/include")
    # NDK shaderc requires building from source via CMake
    # Use prebuilt libs path if available
    find_library(SHADERC_LIB shaderc_combined
        HINTS "${NDK_SHADERC}/libs/${ANDROID_ABI}"
        NO_DEFAULT_PATH
    )
endif()

# Fallback to plugin's vendored shaderc
if(NOT SHADERC_LIB)
    set(PLUGIN_SHADERC "${CMAKE_CURRENT_SOURCE_DIR}/../../../../third_party/shaderc")
    target_include_directories(flutter_vulkan_plugin PRIVATE "${PLUGIN_SHADERC}/include")
    find_library(SHADERC_LIB shaderc
        HINTS "${PLUGIN_SHADERC}/lib/${ANDROID_ABI}"
        NO_DEFAULT_PATH
    )
endif()

if(NOT SHADERC_LIB)
    # Last resort: use NDK's built-in shaderc via add_subdirectory
    if(EXISTS "${NDK_SHADERC}/CMakeLists.txt")
        add_subdirectory("${NDK_SHADERC}" shaderc_build)
        target_link_libraries(flutter_vulkan_plugin PRIVATE shaderc)
    else()
        message(FATAL_ERROR "Could not find shaderc library. Provide it in third_party/shaderc/lib/${ANDROID_ABI}/ or ensure your NDK includes shaderc.")
    endif()
else()
    target_link_libraries(flutter_vulkan_plugin PRIVATE ${SHADERC_LIB})
endif()
