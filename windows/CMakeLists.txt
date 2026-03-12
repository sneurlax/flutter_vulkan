cmake_minimum_required(VERSION 3.14)

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

set(RUST_LIB "${RUST_DIR}/target/${RUST_PROFILE}/flutter_vulkan_native.lib")

add_custom_command(
    OUTPUT "${RUST_LIB}"
    COMMAND cargo build ${RUST_BUILD_FLAG}
            --manifest-path="${RUST_DIR}/Cargo.toml"
    COMMENT "Building Rust native library for Windows (${RUST_PROFILE})..."
    WORKING_DIRECTORY "${RUST_DIR}"
)

add_custom_target(rust_build DEPENDS "${RUST_LIB}")

# ---------------------------------------------------------------------------
# Flutter plugin glue (C++ – talks to the Flutter Windows embedding API)
# ---------------------------------------------------------------------------
add_library(${PLUGIN_NAME} SHARED
  "flutter_vulkan_plugin.cpp"
  "flutter_vulkan_plugin.def"
)
add_dependencies(${PLUGIN_NAME} rust_build)

apply_standard_settings(${PLUGIN_NAME})

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden
  LINK_FLAGS "/DEF:\"${CMAKE_CURRENT_SOURCE_DIR}/flutter_vulkan_plugin.def\""
)

target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)

target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
target_include_directories(${PLUGIN_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)

target_link_libraries(${PLUGIN_NAME} PRIVATE flutter flutter_wrapper_plugin)
target_link_libraries(${PLUGIN_NAME} PRIVATE "${RUST_LIB}")

# ---------------------------------------------------------------------------
# Windows system libraries required by the Rust static lib at link time.
#
# Standard SDK libs (MSVC finds these automatically):
#   - opengl32   : WGL context management (wgpu GLES-over-WGL backend)
#   - d3d11/d3d12/dxgi : DirectX backends used by wgpu
#   - d3dcompiler: FXC shader compilation (wgpu dx11/dx12 back-end)
#   - propsys    : PropVariant / BSTR helpers (windows-core crate)
#   - runtimeobject: WinRT error origination (windows-result crate)
#   - dbghelp    : stack-trace helpers (Rust runtime)
#   - kernel32/ntdll/userenv/ws2_32/bcrypt: Rust std-lib runtime
#
# Non-SDK lib from the Rust `windows` crate (lives in cargo registry):
#   windows.<ver>.lib aggregates import stubs for all Windows APIs the
#   `windows` crate exposes.  Locate it via glob so the version doesn't
#   have to be hard-coded.
# ---------------------------------------------------------------------------

# Standard Windows SDK import libraries.
target_link_libraries(${PLUGIN_NAME} PRIVATE
  opengl32 d3d11 d3d12 dxgi dxguid d3dcompiler
  propsys runtimeobject
  dbghelp kernel32 ntdll userenv ws2_32 bcrypt
)

# Locate the windows.<ver>.lib shipped with the Rust `windows` crate.
if(DEFINED ENV{CARGO_HOME})
  set(_CARGO_HOME "$ENV{CARGO_HOME}")
else()
  set(_CARGO_HOME "$ENV{USERPROFILE}/.cargo")
endif()

file(GLOB _WINDOWS_CRATE_LIBS
  "${_CARGO_HOME}/registry/src/*/windows_x86_64_msvc-*/lib/windows.*.lib"
)
if(_WINDOWS_CRATE_LIBS)
  list(GET _WINDOWS_CRATE_LIBS 0 _WINDOWS_CRATE_LIB)
  message(STATUS "flutter_vulkan: linking ${_WINDOWS_CRATE_LIB}")
  target_link_libraries(${PLUGIN_NAME} PRIVATE "${_WINDOWS_CRATE_LIB}")
else()
  message(WARNING
    "flutter_vulkan: could not find windows.<ver>.lib in cargo registry. "
    "Set CARGO_HOME if it is in a non-default location.")
endif()

set(flutter_vulkan_bundled_libraries
  "$<TARGET_FILE:${PLUGIN_NAME}>"
  PARENT_SCOPE
)
