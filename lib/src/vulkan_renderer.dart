import 'dart:typed_data';

import 'package:flutter/material.dart';

enum PointerEventType {
  onPointerDown,
  onPointerMove,
  onPointerUp,
}

enum UniformType {
  uniformBool,
  uniformInt,
  uniformFloat,
  uniformVec2,
  uniformVec3,
  uniformVec4,
  uniformMat2,
  uniformMat3,
  uniformMat4,
  uniformSampler2D,
}

abstract class VulkanRenderer {
  bool rendererStatus();
  Size getTextureSize();
  void startThread();
  void stopThread();
  String setShader(bool isContinuous, String vertexShader, String fragmentShader);
  String setShaderToy(String fragmentShader);
  String getVertexShader();
  String getFragmentShader();
  void addShaderToyUniforms();
  void setMousePosition(Offset startingPos, Offset pos, PointerEventType eventType, Size twSize);
  double getFps();

  bool addBoolUniform(String name, bool val);
  bool addIntUniform(String name, int val);
  bool addFloatUniform(String name, double val);
  bool addVec2Uniform(String name, List<double> val);
  bool addVec3Uniform(String name, List<double> val);
  bool addVec4Uniform(String name, List<double> val);
  bool addMat2Uniform(String name, List<double> val);
  bool addMat3Uniform(String name, List<double> val);
  bool addMat4Uniform(String name, List<double> val);
  bool addSampler2DUniform(String name, int width, int height, Uint8List val);

  bool setBoolUniform(String name, bool val);
  bool setIntUniform(String name, int val);
  bool setFloatUniform(String name, double val);
  bool setVec2Uniform(String name, List<double> val);
  bool setVec3Uniform(String name, List<double> val);
  bool setVec4Uniform(String name, List<double> val);
  bool setMat2Uniform(String name, List<double> val);
  bool setMat3Uniform(String name, List<double> val);
  bool setMat4Uniform(String name, List<double> val);
  bool setSampler2DUniform(String name, Uint8List val);

  bool replaceSampler2DUniform(String name, int width, int height, Uint8List val);
  bool removeUniform(String name);
}
