cmake_minimum_required(VERSION 3.10)
project("flutter_vulkan_plugin" LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)

# JNI bridge only
add_library(flutter_vulkan_plugin SHARED flutter_vulkan_jni.cpp)

# Link Rust library - it needs to be pre-built for the target ABI
# The Rust library exports the same C FFI symbols
set(RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../../rust")

# For Android, Rust must be cross-compiled. We expect the .so at:
# rust/target/<triple>/release/libflutter_vulkan_plugin.so
# where triple is: aarch64-linux-android or x86_64-linux-android
if(ANDROID_ABI STREQUAL "arm64-v8a")
    set(RUST_TARGET "aarch64-linux-android")
elseif(ANDROID_ABI STREQUAL "x86_64")
    set(RUST_TARGET "x86_64-linux-android")
endif()

set(RUST_LIB "${RUST_DIR}/target/${RUST_TARGET}/release/libflutter_vulkan_plugin.so")

# Build Rust (requires cargo-ndk or manual cross-compilation)
add_custom_command(
    OUTPUT "${RUST_LIB}"
    COMMAND cargo ndk -t ${ANDROID_ABI} build --release --manifest-path="${RUST_DIR}/Cargo.toml"
    COMMENT "Building Rust library for Android ${ANDROID_ABI}..."
    WORKING_DIRECTORY "${RUST_DIR}"
)
add_custom_target(rust_build DEPENDS "${RUST_LIB}")
add_dependencies(flutter_vulkan_plugin rust_build)

target_link_libraries(flutter_vulkan_plugin PRIVATE "${RUST_LIB}")
target_link_libraries(flutter_vulkan_plugin PRIVATE android log)

if(CMAKE_BUILD_TYPE STREQUAL "Release")
    target_compile_options(flutter_vulkan_plugin PRIVATE -O2 -DNDEBUG -fvisibility=hidden)
    target_link_options(flutter_vulkan_plugin PRIVATE -Wl,--gc-sections -Wl,-s)
endif()
