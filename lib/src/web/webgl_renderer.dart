import 'dart:typed_data';
import 'dart:js_interop';

import 'package:flutter/material.dart';
import 'package:web/web.dart' as web;
import 'dart:ui_web' as ui_web;

import '../vulkan_renderer.dart';

typedef _GL = web.WebGL2RenderingContext;

class _UniformInfo {
  final UniformType type;
  final web.WebGLUniformLocation location;
  dynamic value;

  _UniformInfo({required this.type, required this.location, this.value});
}

class _Sampler2DInfo {
  final web.WebGLUniformLocation location;
  final web.WebGLTexture texture;
  final int textureUnit;
  int width;
  int height;

  _Sampler2DInfo({
    required this.location,
    required this.texture,
    required this.textureUnit,
    required this.width,
    required this.height,
  });
}

class WebGLRenderer implements VulkanRenderer {
  static final WebGLRenderer instance = WebGLRenderer._();
  WebGLRenderer._();

  web.HTMLCanvasElement? _canvas;
  web.WebGL2RenderingContext? _gl;
  web.WebGLProgram? _program;
  web.WebGLBuffer? _vertexBuffer;
  int? _animFrameId;
  bool _running = false;
  int _canvasWidth = 0;
  int _canvasHeight = 0;

  final Map<String, _UniformInfo> _uniforms = {};
  final Map<String, _Sampler2DInfo> _samplers = {};
  int _nextTextureUnit = 0;

  web.WebGLUniformLocation? _iTimeLoc;
  web.WebGLUniformLocation? _iResolutionLoc;
  web.WebGLUniformLocation? _iMouseLoc;
  web.WebGLUniformLocation? _iFrameLoc;

  double _startTime = 0;
  int _frame = 0;
  double _mouseX = 0;
  double _mouseY = 0;
  double _mouseZ = 0;
  double _mouseW = 0;

  double _lastFrameTime = 0;
  double _fps = 0;
  int _frameCount = 0;
  double _fpsAccum = 0;

  String? _currentVertexSrc;
  String? _currentFragmentSrc;
  bool _isShaderToy = false;

  static int _nextViewId = 9000;

  late final JSFunction _rafCallback = _onAnimFrame.toJS;

  int createSurface(int width, int height) {
    final viewId = _nextViewId++;
    final viewType = 'flutter_vulkan_$viewId';

    _canvasWidth = width;
    _canvasHeight = height;

    _canvas = web.HTMLCanvasElement()
      ..width = width
      ..height = height;
    _canvas!.style.width = '100%';
    _canvas!.style.height = '100%';

    final ctx = _canvas!.getContext('webgl2');
    if (ctx == null) {
      throw StateError('WebGL2 not supported');
    }
    _gl = ctx as _GL;
    _initQuadBuffer();

    final canvas = _canvas!;
    ui_web.platformViewRegistry.registerViewFactory(viewType, (int id) {
      return canvas;
    });

    return viewId;
  }

  void _initQuadBuffer() {
    final gl = _gl!;
    _vertexBuffer = gl.createBuffer();
    gl.bindBuffer(_GL.ARRAY_BUFFER, _vertexBuffer);
    final vertices = Float32List.fromList([
      -1, -1,
       1, -1,
      -1,  1,
      -1,  1,
       1, -1,
       1,  1,
    ]);
    gl.bufferData(_GL.ARRAY_BUFFER, vertices.toJS, _GL.STATIC_DRAW);
  }

  web.WebGLShader? _compileShader(int type, String source) {
    final gl = _gl!;
    final shader = gl.createShader(type);
    if (shader == null) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    final success = (gl.getShaderParameter(shader, _GL.COMPILE_STATUS) as JSBoolean).toDart;
    if (!success) {
      final log = gl.getShaderInfoLog(shader);
      gl.deleteShader(shader);
      throw StateError('Shader compile error: $log');
    }
    return shader;
  }

  String _linkProgram(String vertexSrc, String fragmentSrc) {
    final gl = _gl!;

    web.WebGLShader vs, fs;
    try {
      vs = _compileShader(_GL.VERTEX_SHADER, vertexSrc)!;
    } catch (e) {
      return 'Vertex shader error: $e';
    }
    try {
      fs = _compileShader(_GL.FRAGMENT_SHADER, fragmentSrc)!;
    } catch (e) {
      gl.deleteShader(vs);
      return 'Fragment shader error: $e';
    }

    if (_program != null) {
      gl.deleteProgram(_program);
    }

    _program = gl.createProgram();
    gl.attachShader(_program!, vs);
    gl.attachShader(_program!, fs);
    gl.linkProgram(_program!);

    gl.deleteShader(vs);
    gl.deleteShader(fs);

    final success = (gl.getProgramParameter(_program!, _GL.LINK_STATUS) as JSBoolean).toDart;
    if (!success) {
      final log = gl.getProgramInfoLog(_program!);
      gl.deleteProgram(_program);
      _program = null;
      return 'Link error: $log';
    }

    gl.useProgram(_program!);

    final posLoc = gl.getAttribLocation(_program!, 'a_position');
    if (posLoc >= 0) {
      gl.bindBuffer(_GL.ARRAY_BUFFER, _vertexBuffer);
      gl.enableVertexAttribArray(posLoc);
      gl.vertexAttribPointer(posLoc, 2, _GL.FLOAT, false, 0, 0);
    }

    return '';
  }

  @override
  String setShader(bool isContinuous, String vertexShader, String fragmentShader) {
    if (_gl == null) return 'WebGL context not initialized';

    _isShaderToy = false;
    _currentVertexSrc = vertexShader;
    _currentFragmentSrc = fragmentShader;

    final err = _linkProgram(vertexShader, fragmentShader);
    if (err.isNotEmpty) return err;

    _iTimeLoc = null;
    _iResolutionLoc = null;
    _iMouseLoc = null;
    _iFrameLoc = null;

    _relookupUniforms();
    return '';
  }

  @override
  String setShaderToy(String fragmentShader) {
    if (_gl == null) return 'WebGL context not initialized';

    _isShaderToy = true;

    const vertexSrc = '#version 300 es\n'
        'in vec2 a_position;\n'
        'void main() {\n'
        '    gl_Position = vec4(a_position, 0.0, 1.0);\n'
        '}\n';

    final fragmentSrc = '#version 300 es\n'
        'precision highp float;\n'
        'uniform float iTime;\n'
        'uniform vec3 iResolution;\n'
        'uniform vec4 iMouse;\n'
        'uniform int iFrame;\n'
        'uniform sampler2D iChannel0;\n'
        'uniform sampler2D iChannel1;\n'
        'uniform sampler2D iChannel2;\n'
        'uniform sampler2D iChannel3;\n'
        'out vec4 fragColor;\n\n'
        '$fragmentShader\n\n'
        'void main() {\n'
        '    mainImage(fragColor, gl_FragCoord.xy);\n'
        '}\n';

    _currentVertexSrc = vertexSrc;
    _currentFragmentSrc = fragmentSrc;

    final err = _linkProgram(vertexSrc, fragmentSrc);
    if (err.isNotEmpty) return err;

    _iTimeLoc = _gl!.getUniformLocation(_program!, 'iTime');
    _iResolutionLoc = _gl!.getUniformLocation(_program!, 'iResolution');
    _iMouseLoc = _gl!.getUniformLocation(_program!, 'iMouse');
    _iFrameLoc = _gl!.getUniformLocation(_program!, 'iFrame');

    _relookupUniforms();
    addShaderToyUniforms();
    return '';
  }

  void _relookupUniforms() {
    if (_program == null || _gl == null) return;
    final gl = _gl!;

    for (final entry in _uniforms.entries) {
      final loc = gl.getUniformLocation(_program!, entry.key);
      if (loc != null) {
        _uniforms[entry.key] = _UniformInfo(
          type: entry.value.type,
          location: loc,
          value: entry.value.value,
        );
      }
    }

    for (final entry in _samplers.entries) {
      final loc = gl.getUniformLocation(_program!, entry.key);
      if (loc != null) {
        _samplers[entry.key] = _Sampler2DInfo(
          location: loc,
          texture: entry.value.texture,
          textureUnit: entry.value.textureUnit,
          width: entry.value.width,
          height: entry.value.height,
        );
      }
    }
  }

  void _onAnimFrame(double timestamp) {
    if (!_running || _gl == null || _program == null) return;

    final now = timestamp / 1000.0;

    if (_lastFrameTime > 0) {
      final dt = now - _lastFrameTime;
      _frameCount++;
      _fpsAccum += dt;
      if (_fpsAccum >= 0.5) {
        _fps = _frameCount / _fpsAccum;
        _frameCount = 0;
        _fpsAccum = 0;
      }
    }
    _lastFrameTime = now;

    final gl = _gl!;
    final elapsed = now - _startTime;

    gl.viewport(0, 0, _canvasWidth, _canvasHeight);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(_GL.COLOR_BUFFER_BIT);

    gl.useProgram(_program!);

    if (_isShaderToy) {
      if (_iTimeLoc != null) gl.uniform1f(_iTimeLoc!, elapsed);
      if (_iResolutionLoc != null) {
        gl.uniform3f(_iResolutionLoc!, _canvasWidth.toDouble(), _canvasHeight.toDouble(), 1.0);
      }
      if (_iMouseLoc != null) gl.uniform4f(_iMouseLoc!, _mouseX, _mouseY, _mouseZ, _mouseW);
      if (_iFrameLoc != null) gl.uniform1i(_iFrameLoc!, _frame);
    }

    for (final entry in _uniforms.entries) {
      _applyUniform(entry.value);
    }

    for (final entry in _samplers.entries) {
      final info = entry.value;
      gl.activeTexture(_GL.TEXTURE0 + info.textureUnit);
      gl.bindTexture(_GL.TEXTURE_2D, info.texture);
      gl.uniform1i(info.location, info.textureUnit);
    }

    gl.drawArrays(_GL.TRIANGLES, 0, 6);
    _frame++;

    _animFrameId = web.window.requestAnimationFrame(_rafCallback);
  }

  void _applyUniform(_UniformInfo info) {
    final gl = _gl!;
    final val = info.value;
    if (val == null) return;

    switch (info.type) {
      case UniformType.uniformBool:
        gl.uniform1i(info.location, (val as bool) ? 1 : 0);
      case UniformType.uniformInt:
        gl.uniform1i(info.location, val as int);
      case UniformType.uniformFloat:
        gl.uniform1f(info.location, val as double);
      case UniformType.uniformVec2:
        final v = val as List<double>;
        gl.uniform2f(info.location, v[0], v[1]);
      case UniformType.uniformVec3:
        final v = val as List<double>;
        gl.uniform3f(info.location, v[0], v[1], v[2]);
      case UniformType.uniformVec4:
        final v = val as List<double>;
        gl.uniform4f(info.location, v[0], v[1], v[2], v[3]);
      case UniformType.uniformMat2:
        final v = val as List<double>;
        gl.uniformMatrix2fv(info.location, false, Float32List.fromList(v).toJS);
      case UniformType.uniformMat3:
        final v = val as List<double>;
        gl.uniformMatrix3fv(info.location, false, Float32List.fromList(v).toJS);
      case UniformType.uniformMat4:
        final v = val as List<double>;
        gl.uniformMatrix4fv(info.location, false, Float32List.fromList(v).toJS);
      case UniformType.uniformSampler2D:
        break;
    }
  }

  @override
  void startThread() {
    if (_running) return;
    _running = true;
    _startTime = web.window.performance.now() / 1000.0;
    _frame = 0;
    _frameCount = 0;
    _fpsAccum = 0;
    _lastFrameTime = 0;
    _animFrameId = web.window.requestAnimationFrame(_rafCallback);
  }

  @override
  void stopThread() {
    _running = false;
    if (_animFrameId != null) {
      web.window.cancelAnimationFrame(_animFrameId!);
      _animFrameId = null;
    }
  }

  @override
  bool rendererStatus() => _running && _program != null;

  @override
  Size getTextureSize() => Size(_canvasWidth.toDouble(), _canvasHeight.toDouble());

  @override
  String getVertexShader() => _currentVertexSrc ?? '';

  @override
  String getFragmentShader() => _currentFragmentSrc ?? '';

  @override
  void addShaderToyUniforms() {
    for (int i = 0; i < 4; i++) {
      final name = 'iChannel$i';
      if (!_samplers.containsKey(name)) {
        addSampler2DUniform(name, 1, 1, Uint8List.fromList([0, 0, 0, 255]));
      }
    }
  }

  @override
  void setMousePosition(Offset startingPos, Offset pos, PointerEventType eventType, Size twSize) {
    if (twSize.width <= 0 || twSize.height <= 0) return;
    final nx = pos.dx / twSize.width * _canvasWidth;
    final ny = (_canvasHeight - (pos.dy / twSize.height * _canvasHeight));

    _mouseX = nx;
    _mouseY = ny;

    if (eventType == PointerEventType.onPointerDown || eventType == PointerEventType.onPointerMove) {
      final sx = startingPos.dx / twSize.width * _canvasWidth;
      final sy = (_canvasHeight - (startingPos.dy / twSize.height * _canvasHeight));
      _mouseZ = sx;
      _mouseW = sy;
    } else {
      _mouseZ = -_mouseZ.abs();
      _mouseW = -_mouseW.abs();
    }
  }

  @override
  double getFps() => _fps;

  bool _addUniform(String name, UniformType type, dynamic value) {
    if (_program == null || _gl == null) return false;
    final loc = _gl!.getUniformLocation(_program!, name);
    if (loc == null) return false;
    _uniforms[name] = _UniformInfo(type: type, location: loc, value: value);
    return true;
  }

  @override
  bool addBoolUniform(String name, bool val) => _addUniform(name, UniformType.uniformBool, val);
  @override
  bool addIntUniform(String name, int val) => _addUniform(name, UniformType.uniformInt, val);
  @override
  bool addFloatUniform(String name, double val) => _addUniform(name, UniformType.uniformFloat, val);
  @override
  bool addVec2Uniform(String name, List<double> val) => _addUniform(name, UniformType.uniformVec2, List<double>.from(val));
  @override
  bool addVec3Uniform(String name, List<double> val) => _addUniform(name, UniformType.uniformVec3, List<double>.from(val));
  @override
  bool addVec4Uniform(String name, List<double> val) => _addUniform(name, UniformType.uniformVec4, List<double>.from(val));
  @override
  bool addMat2Uniform(String name, List<double> val) => _addUniform(name, UniformType.uniformMat2, List<double>.from(val));
  @override
  bool addMat3Uniform(String name, List<double> val) => _addUniform(name, UniformType.uniformMat3, List<double>.from(val));
  @override
  bool addMat4Uniform(String name, List<double> val) => _addUniform(name, UniformType.uniformMat4, List<double>.from(val));

  @override
  bool addSampler2DUniform(String name, int width, int height, Uint8List val) {
    if (_program == null || _gl == null) return false;
    final gl = _gl!;
    final loc = gl.getUniformLocation(_program!, name);
    if (loc == null) return false;

    final unit = _nextTextureUnit++;
    final tex = gl.createTexture()!;
    gl.activeTexture(_GL.TEXTURE0 + unit);
    gl.bindTexture(_GL.TEXTURE_2D, tex);
    gl.texImage2D(
      _GL.TEXTURE_2D, 0, _GL.RGBA,
      width.toJS, height.toJS, 0.toJS,
      _GL.RGBA, _GL.UNSIGNED_BYTE, val.toJS,
    );
    gl.generateMipmap(_GL.TEXTURE_2D);
    gl.texParameteri(_GL.TEXTURE_2D, _GL.TEXTURE_MIN_FILTER, _GL.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(_GL.TEXTURE_2D, _GL.TEXTURE_MAG_FILTER, _GL.LINEAR);
    gl.texParameteri(_GL.TEXTURE_2D, _GL.TEXTURE_WRAP_S, _GL.CLAMP_TO_EDGE);
    gl.texParameteri(_GL.TEXTURE_2D, _GL.TEXTURE_WRAP_T, _GL.CLAMP_TO_EDGE);

    _samplers[name] = _Sampler2DInfo(
      location: loc,
      texture: tex,
      textureUnit: unit,
      width: width,
      height: height,
    );
    return true;
  }

  bool _setUniform(String name, dynamic value) {
    final info = _uniforms[name];
    if (info == null) return false;
    info.value = value;
    return true;
  }

  @override
  bool setBoolUniform(String name, bool val) => _setUniform(name, val);
  @override
  bool setIntUniform(String name, int val) => _setUniform(name, val);
  @override
  bool setFloatUniform(String name, double val) => _setUniform(name, val);
  @override
  bool setVec2Uniform(String name, List<double> val) => _setUniform(name, List<double>.from(val));
  @override
  bool setVec3Uniform(String name, List<double> val) => _setUniform(name, List<double>.from(val));
  @override
  bool setVec4Uniform(String name, List<double> val) => _setUniform(name, List<double>.from(val));
  @override
  bool setMat2Uniform(String name, List<double> val) => _setUniform(name, List<double>.from(val));
  @override
  bool setMat3Uniform(String name, List<double> val) => _setUniform(name, List<double>.from(val));
  @override
  bool setMat4Uniform(String name, List<double> val) => _setUniform(name, List<double>.from(val));

  @override
  bool setSampler2DUniform(String name, Uint8List val) {
    final info = _samplers[name];
    if (info == null || _gl == null) return false;
    final gl = _gl!;
    gl.activeTexture(_GL.TEXTURE0 + info.textureUnit);
    gl.bindTexture(_GL.TEXTURE_2D, info.texture);
    gl.texSubImage2D(
      _GL.TEXTURE_2D, 0, 0, 0,
      info.width.toJS, info.height.toJS, _GL.RGBA.toJS,
      _GL.UNSIGNED_BYTE, val.toJS,
    );
    return true;
  }

  @override
  bool replaceSampler2DUniform(String name, int width, int height, Uint8List val) {
    final info = _samplers[name];
    if (info == null || _gl == null) return false;
    final gl = _gl!;
    gl.activeTexture(_GL.TEXTURE0 + info.textureUnit);
    gl.bindTexture(_GL.TEXTURE_2D, info.texture);
    gl.texImage2D(
      _GL.TEXTURE_2D, 0, _GL.RGBA,
      width.toJS, height.toJS, 0.toJS,
      _GL.RGBA, _GL.UNSIGNED_BYTE, val.toJS,
    );
    gl.generateMipmap(_GL.TEXTURE_2D);
    info.width = width;
    info.height = height;
    return true;
  }

  @override
  bool removeUniform(String name) {
    if (_uniforms.containsKey(name)) {
      _uniforms.remove(name);
      return true;
    }
    if (_samplers.containsKey(name)) {
      final info = _samplers.remove(name)!;
      _gl?.deleteTexture(info.texture);
      return true;
    }
    return false;
  }
}
