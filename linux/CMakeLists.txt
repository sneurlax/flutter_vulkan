cmake_minimum_required(VERSION 3.10)

set(PROJECT_NAME "flutter_vulkan")
project(${PROJECT_NAME} LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(PLUGIN_NAME "flutter_vulkan_plugin")

set(PLUGIN_SOURCES
  flutter_vulkan_plugin.cc
  fl_my_texture.cc
  ../src/ffi.cpp
  ../src/renderer.cpp
  ../src/shader.cpp
  ../src/uniform_queue.cpp
  ../src/sampler2d.cpp
  ../src/vulkan_context.cpp
)

add_library(${PLUGIN_NAME} SHARED ${PLUGIN_SOURCES})

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)

target_compile_options(${PLUGIN_NAME} PRIVATE -Wall -Wno-error)

target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
target_include_directories(${PLUGIN_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
  "${CMAKE_CURRENT_SOURCE_DIR}/../src"
  "${CMAKE_CURRENT_SOURCE_DIR}/../third_party/shaderc/include"
)

target_link_libraries(${PLUGIN_NAME} PRIVATE flutter)
target_link_libraries(${PLUGIN_NAME} PRIVATE PkgConfig::GTK)

# Vulkan
find_package(Vulkan REQUIRED)
target_link_libraries(${PLUGIN_NAME} PRIVATE Vulkan::Vulkan)

# shaderc - vendored in third_party/
set(SHADERC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../third_party/shaderc")
find_library(SHADERC_LIB shaderc
  HINTS "${SHADERC_DIR}/lib"
  NO_DEFAULT_PATH
)
if(NOT SHADERC_LIB)
  # Fallback to system
  find_library(SHADERC_LIB shaderc HINTS /usr/lib /usr/lib/x86_64-linux-gnu)
endif()
if(NOT SHADERC_LIB)
  find_library(SHADERC_LIB shaderc_combined HINTS /usr/lib /usr/lib/x86_64-linux-gnu)
endif()
if(NOT SHADERC_LIB)
  message(FATAL_ERROR "Could not find shaderc library.")
endif()
target_link_libraries(${PLUGIN_NAME} PRIVATE ${SHADERC_LIB})

# Bundle the shaderc shared library with the app
set(flutter_vulkan_bundled_libraries
  "${SHADERC_DIR}/lib/libshaderc.so.1"
  PARENT_SCOPE
)
