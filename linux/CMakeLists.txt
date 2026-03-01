cmake_minimum_required(VERSION 3.10)

set(PROJECT_NAME "flutter_vulkan")
project(${PROJECT_NAME} LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(PLUGIN_NAME "flutter_vulkan_plugin")
set(RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../rust")

# ---------------------------------------------------------------------------
# Build Rust native library via cargo
# ---------------------------------------------------------------------------
if(CMAKE_BUILD_TYPE MATCHES "Debug")
  set(RUST_BUILD_FLAG "")
  set(RUST_PROFILE "debug")
else()
  set(RUST_BUILD_FLAG "--release")
  set(RUST_PROFILE "release")
endif()

set(RUST_LIB "${RUST_DIR}/target/${RUST_PROFILE}/libflutter_vulkan_native.a")

add_custom_command(
    OUTPUT "${RUST_LIB}"
    COMMAND cargo build ${RUST_BUILD_FLAG} --manifest-path="${RUST_DIR}/Cargo.toml"
    COMMENT "Building Rust native library (${RUST_PROFILE})..."
    WORKING_DIRECTORY "${RUST_DIR}"
)
add_custom_target(rust_build DEPENDS "${RUST_LIB}")

# ---------------------------------------------------------------------------
# Flutter plugin glue (C++ – talks to the Flutter Linux embedding API)
# ---------------------------------------------------------------------------
set(PLUGIN_SOURCES
  flutter_vulkan_plugin.cc
  fl_my_texture.cc
)

add_library(${PLUGIN_NAME} SHARED ${PLUGIN_SOURCES})
add_dependencies(${PLUGIN_NAME} rust_build)

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)

target_compile_options(${PLUGIN_NAME} PRIVATE -Wall -Wno-error)

target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
target_include_directories(${PLUGIN_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)

target_link_libraries(${PLUGIN_NAME} PRIVATE flutter)
target_link_libraries(${PLUGIN_NAME} PRIVATE PkgConfig::GTK)
target_link_libraries(${PLUGIN_NAME} PRIVATE "${RUST_LIB}")

# Rust dependencies that the static library needs at link time
target_link_libraries(${PLUGIN_NAME} PRIVATE dl pthread m)
