import 'dart:ffi' as ffi;
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
import 'package:flutter/material.dart';

import 'vulkan_renderer.dart';

export 'vulkan_renderer.dart' show PointerEventType, UniformType;

class FlutterVulkanFfi implements VulkanRenderer {
  final ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
      _lookup;

  FlutterVulkanFfi(ffi.DynamicLibrary dynamicLibrary)
      : _lookup = dynamicLibrary.lookup;

  FlutterVulkanFfi.fromLookup(
      ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
          lookup)
      : _lookup = lookup;

  @override
  bool rendererStatus() {
    return _rendererStatus() == 0 ? false : true;
  }

  late final _rendererStatusPtr =
      _lookup<ffi.NativeFunction<ffi.Int Function()>>('rendererStatus');
  late final _rendererStatus = _rendererStatusPtr.asFunction<int Function()>();

  @override
  Size getTextureSize() {
    ffi.Pointer<ffi.Int32> w = calloc(ffi.sizeOf<ffi.Int32>());
    ffi.Pointer<ffi.Int32> h = calloc(ffi.sizeOf<ffi.Int32>());

    _textureSize(w, h);
    Size size = Size(w.value.toDouble(), h.value.toDouble());

    calloc.free(w);
    calloc.free(h);
    return size;
  }

  late final _textureSizePtr = _lookup<
      ffi.NativeFunction<
          ffi.Int Function(ffi.Pointer<ffi.Int32>,
              ffi.Pointer<ffi.Int32>)>>('getTextureSize');
  late final _textureSize = _textureSizePtr.asFunction<
      int Function(ffi.Pointer<ffi.Int32>, ffi.Pointer<ffi.Int32>)>();

  @override
  void startThread() {
    return _startThread();
  }

  late final _startThreadPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function()>>('startThread');
  late final _startThread = _startThreadPtr.asFunction<void Function()>();

  @override
  void stopThread() {
    return _stopThread();
  }

  late final _stopThreadPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function()>>('stopThread');
  late final _stopThread = _stopThreadPtr.asFunction<void Function()>();

  @override
  String setShader(
    bool isContinuous,
    String vertexShader,
    String fragmentShader,
  ) {
    ffi.Pointer<ffi.Char> err = _setShader(
      isContinuous ? 1 : 0,
      vertexShader.toNativeUtf8().cast<ffi.Char>(),
      fragmentShader.toNativeUtf8().cast<ffi.Char>(),
    );
    String ret = err.cast<Utf8>().toDartString();
    return ret;
  }

  late final _setShaderPtr = _lookup<
      ffi.NativeFunction<
          ffi.Pointer<ffi.Char> Function(ffi.Int, ffi.Pointer<ffi.Char>,
              ffi.Pointer<ffi.Char>)>>('setShader');
  late final _setShader = _setShaderPtr.asFunction<
      ffi.Pointer<ffi.Char> Function(
          int, ffi.Pointer<ffi.Char>, ffi.Pointer<ffi.Char>)>();

  @override
  String setShaderToy(String fragmentShader) {
    return _setShaderToy(
      fragmentShader.toNativeUtf8().cast<ffi.Char>(),
    ).cast<Utf8>().toDartString();
  }

  late final _setShaderToyPtr = _lookup<
      ffi.NativeFunction<
          ffi.Pointer<ffi.Char> Function(
              ffi.Pointer<ffi.Char>)>>('setShaderToy');
  late final _setShaderToy = _setShaderToyPtr
      .asFunction<ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.Char>)>();

  @override
  String getVertexShader() {
    ffi.Pointer<ffi.Char> vs = _getVertexShader();
    return vs.cast<Utf8>().toDartString();
  }

  late final _getVertexShaderPtr =
      _lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function()>>(
          'getVertexShader');
  late final _getVertexShader =
      _getVertexShaderPtr.asFunction<ffi.Pointer<ffi.Char> Function()>();

  @override
  String getFragmentShader() {
    ffi.Pointer<ffi.Char> fs = _getFragmentShader();
    return fs.cast<Utf8>().toDartString();
  }

  late final _getFragmentShaderPtr =
      _lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function()>>(
          'getFragmentShader');
  late final _getFragmentShader =
      _getFragmentShaderPtr.asFunction<ffi.Pointer<ffi.Char> Function()>();

  @override
  void addShaderToyUniforms() {
    return _addShaderToyUniforms();
  }

  late final _addShaderToyUniformsPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function()>>('addShaderToyUniforms');
  late final _addShaderToyUniforms =
      _addShaderToyUniformsPtr.asFunction<void Function()>();

  @override
  void setMousePosition(
    Offset startingPos,
    Offset pos,
    PointerEventType eventType,
    Size twSize,
  ) {
    return _setMousePosition(
      pos.dx,
      pos.dy,
      eventType == PointerEventType.onPointerDown ||
              eventType == PointerEventType.onPointerMove
          ? startingPos.dx
          : -startingPos.dx,
      -startingPos.dy,
      twSize.width,
      twSize.height,
    );
  }

  late final _setMousePositionPtr = _lookup<
      ffi.NativeFunction<
          ffi.Void Function(ffi.Double, ffi.Double, ffi.Double, ffi.Double,
              ffi.Double, ffi.Double)>>('setMousePosition');
  late final _setMousePosition = _setMousePositionPtr.asFunction<
      void Function(double, double, double, double, double, double)>();

  @override
  double getFps() {
    return _getFps();
  }

  late final _getFpsPtr =
      _lookup<ffi.NativeFunction<ffi.Double Function()>>('getFPS');
  late final _getFps = _getFpsPtr.asFunction<double Function()>();

  // Uniform add methods
  @override
  bool addBoolUniform(String name, bool val) {
    ffi.Pointer<ffi.Bool> valT = calloc(ffi.sizeOf<ffi.Bool>());
    valT.value = val;
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformBool.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addIntUniform(String name, int val) {
    ffi.Pointer<ffi.Int32> valT = calloc(ffi.sizeOf<ffi.Int32>());
    valT.value = val;
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformInt.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addFloatUniform(String name, double val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>());
    valT.value = val;
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformFloat.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addVec2Uniform(String name, List<double> val) {
    assert(val.length == 2);
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 2);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformVec2.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addVec3Uniform(String name, List<double> val) {
    assert(val.length == 3);
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 3);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformVec3.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addVec4Uniform(String name, List<double> val) {
    assert(val.length == 4);
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 4);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformVec4.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addMat2Uniform(String name, List<double> val) {
    assert(val.length == 4);
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 4);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformMat2.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addMat3Uniform(String name, List<double> val) {
    assert(val.length == 9);
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 9);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformMat3.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool addMat4Uniform(String name, List<double> val) {
    assert(val.length == 16);
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 16);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      UniformType.uniformMat4.index,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  late final _addUniformPtr = _lookup<
      ffi.NativeFunction<
          ffi.Int Function(ffi.Pointer<ffi.Char>, ffi.Int32,
              ffi.Pointer<ffi.Void>)>>('addUniform');
  late final _addUniform = _addUniformPtr.asFunction<
      int Function(ffi.Pointer<ffi.Char>, int, ffi.Pointer<ffi.Void>)>();

  @override
  bool removeUniform(String name) {
    int ret = _removeUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
    );
    return ret == 0 ? false : true;
  }

  late final _removeUniformPtr =
      _lookup<ffi.NativeFunction<ffi.Int Function(ffi.Pointer<ffi.Char>)>>(
          'removeUniform');
  late final _removeUniform =
      _removeUniformPtr.asFunction<int Function(ffi.Pointer<ffi.Char>)>();

  @override
  bool addSampler2DUniform(
    String name,
    int width,
    int height,
    Uint8List val,
  ) {
    assert(val.length == width * height * 4);
    ffi.Pointer<ffi.Int8> valT = calloc(ffi.sizeOf<ffi.Int8>() * val.length);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _addSampler2DUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      width,
      height,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  late final _addSampler2DUniformPtr = _lookup<
      ffi.NativeFunction<
          ffi.Int Function(ffi.Pointer<ffi.Char>, ffi.Int32, ffi.Int32,
              ffi.Pointer<ffi.Void>)>>('addSampler2DUniform');
  late final _addSampler2DUniform = _addSampler2DUniformPtr.asFunction<
      int Function(ffi.Pointer<ffi.Char>, int, int, ffi.Pointer<ffi.Void>)>();

  @override
  bool replaceSampler2DUniform(
    String name,
    int width,
    int height,
    Uint8List val,
  ) {
    assert(val.length == width * height * 4);
    ffi.Pointer<ffi.Int8> valT = calloc(ffi.sizeOf<ffi.Int8>() * val.length);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _replaceSampler2DUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      width,
      height,
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  late final _replaceSampler2DUniformPtr = _lookup<
      ffi.NativeFunction<
          ffi.Int Function(ffi.Pointer<ffi.Char>, ffi.Int32, ffi.Int32,
              ffi.Pointer<ffi.Void>)>>('replaceSampler2DUniform');
  late final _replaceSampler2DUniform = _replaceSampler2DUniformPtr.asFunction<
      int Function(ffi.Pointer<ffi.Char>, int, int, ffi.Pointer<ffi.Void>)>();

  // Uniform set methods
  @override
  bool setBoolUniform(String name, bool val) {
    ffi.Pointer<ffi.Bool> valT = calloc(ffi.sizeOf<ffi.Bool>());
    valT.value = val;
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setIntUniform(String name, int val) {
    ffi.Pointer<ffi.Int32> valT = calloc(ffi.sizeOf<ffi.Int32>());
    valT.value = val;
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setFloatUniform(String name, double val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>());
    valT.value = val;
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setVec2Uniform(String name, List<double> val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 2);
    valT[0] = val[0];
    valT[1] = val[1];
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setVec3Uniform(String name, List<double> val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 3);
    valT[0] = val[0];
    valT[1] = val[1];
    valT[2] = val[2];
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setVec4Uniform(String name, List<double> val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 4);
    valT[0] = val[0];
    valT[1] = val[1];
    valT[2] = val[2];
    valT[3] = val[3];
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setMat2Uniform(String name, List<double> val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 4);
    for (int i = 0; i < 4; ++i) { valT[i] = val[i]; }
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setMat3Uniform(String name, List<double> val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 9);
    for (int i = 0; i < 9; ++i) { valT[i] = val[i]; }
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setMat4Uniform(String name, List<double> val) {
    ffi.Pointer<ffi.Float> valT = calloc(ffi.sizeOf<ffi.Float>() * 16);
    for (int i = 0; i < 16; ++i) { valT[i] = val[i]; }
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  @override
  bool setSampler2DUniform(String name, Uint8List val) {
    ffi.Pointer<ffi.Int8> valT = calloc(ffi.sizeOf<ffi.Int8>() * val.length);
    for (int i = 0; i < val.length; ++i) {
      valT[i] = val[i];
    }
    int ret = _setUniform(
      name.toNativeUtf8().cast<ffi.Char>(),
      valT.cast<ffi.Void>(),
    );
    calloc.free(valT);
    return ret == 0 ? false : true;
  }

  late final _setUniformPtr = _lookup<
      ffi.NativeFunction<
          ffi.Int Function(
              ffi.Pointer<ffi.Char>, ffi.Pointer<ffi.Void>)>>('setUniform');
  late final _setUniform = _setUniformPtr
      .asFunction<int Function(ffi.Pointer<ffi.Char>, ffi.Pointer<ffi.Void>)>();
}
