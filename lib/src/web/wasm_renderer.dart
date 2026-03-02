import 'dart:typed_data';
import 'dart:js_interop';

import 'package:flutter/material.dart';
import 'package:web/web.dart' as web;
import 'dart:ui_web' as ui_web;

import '../vulkan_renderer.dart';

// ---------------------------------------------------------------------------
// JS interop bindings to the Rust WASM module (wasm-bindgen exports)
// ---------------------------------------------------------------------------

@JS('wasm_bindgen.init_renderer')
external JSPromise<JSAny?> _jsInitRenderer(
    web.HTMLCanvasElement canvas, JSNumber width, JSNumber height);

@JS('wasm_bindgen.start_render_loop')
external void _jsStartRenderLoop();

@JS('wasm_bindgen.stop_render_loop')
external void _jsStopRenderLoop();

@JS('wasm_bindgen.set_shader_toy')
external JSString _jsSetShaderToy(JSString fragmentSrc);

@JS('wasm_bindgen.set_shader')
external JSString _jsSetShader(
    JSBoolean isContinuous, JSString vertexSrc, JSString fragmentSrc);

@JS('wasm_bindgen.get_fps')
external JSNumber _jsGetFps();

@JS('wasm_bindgen.set_mouse_position')
external void _jsSetMousePosition(JSNumber posX, JSNumber posY, JSNumber posZ,
    JSNumber posW, JSNumber twWidth, JSNumber twHeight);

@JS('wasm_bindgen.add_shader_toy_uniforms')
external void _jsAddShaderToyUniforms();

@JS('wasm_bindgen.add_uniform')
external JSBoolean _jsAddUniform(
    JSString name, JSNumber uniformType, JSUint8Array val);

@JS('wasm_bindgen.remove_uniform')
external JSBoolean _jsRemoveUniform(JSString name);

@JS('wasm_bindgen.set_uniform')
external JSBoolean _jsSetUniform(JSString name, JSUint8Array val);

@JS('wasm_bindgen.add_sampler2d_uniform')
external JSBoolean _jsAddSampler2dUniform(
    JSString name, JSNumber width, JSNumber height, JSUint8Array val);

@JS('wasm_bindgen.replace_sampler2d_uniform')
external JSBoolean _jsReplaceSampler2dUniform(
    JSString name, JSNumber width, JSNumber height, JSUint8Array val);

/// Load the wasm-bindgen glue and WASM binary.
///
/// The generated JS glue is expected to be loaded via a `<script>` tag in
/// `index.html` (or via `flutter_bootstrap.js`).  This function calls the
/// default export of that glue to fetch and instantiate the `.wasm` binary.
@JS('wasm_bindgen')
external JSFunction get _wasmBindgenInit;

// ---------------------------------------------------------------------------
// WasmRenderer
// ---------------------------------------------------------------------------

class WasmRenderer implements VulkanRenderer {
  static final WasmRenderer instance = WasmRenderer._();
  WasmRenderer._();

  bool _initialized = false;
  bool _running = false;
  int _canvasWidth = 0;
  int _canvasHeight = 0;

  String? _currentVertexSrc;
  String? _currentFragmentSrc;

  static int _nextViewId = 9000;

  // ------------------------------------------------------------------
  // Lifecycle
  // ------------------------------------------------------------------

  /// Initialise the WASM module and create a canvas-backed platform view.
  ///
  /// This must be awaited before any other method is called.  Returns the
  /// view id that should be passed to [HtmlElementView].
  Future<int> createSurface(int width, int height) async {
    final viewId = _nextViewId++;
    final viewType = 'flutter_vulkan_$viewId';

    _canvasWidth = width;
    _canvasHeight = height;

    // Create the canvas element and temporarily attach it to the DOM.
    // Some browsers require the canvas to be in the document for WebGPU
    // adapter discovery to succeed (compatible_surface check).
    final canvas = web.HTMLCanvasElement()
      ..width = width
      ..height = height;
    canvas.style
      ..width = '100%'
      ..height = '100%'
      ..position = 'absolute'
      ..left = '-9999px';
    web.document.body!.append(canvas);

    // Register with Flutter's platform view registry.  When the
    // HtmlElementView renders, the factory reparents the canvas.
    ui_web.platformViewRegistry.registerViewFactory(viewType, (int id) {
      canvas.style.position = '';
      canvas.style.left = '';
      return canvas;
    });

    // Load the WASM module if this is the first time.
    if (!_initialized) {
      final promise = _wasmBindgenInit.callAsFunction(null) as JSPromise;
      await promise.toDart;
      _initialized = true;
    }

    // Initialise the Rust renderer — pass the canvas element directly.
    final initPromise = _jsInitRenderer(
      canvas,
      width.toJS,
      height.toJS,
    );
    await initPromise.toDart;

    return viewId;
  }

  // ------------------------------------------------------------------
  // VulkanRenderer interface
  // ------------------------------------------------------------------

  @override
  bool rendererStatus() => _running;

  @override
  Size getTextureSize() =>
      Size(_canvasWidth.toDouble(), _canvasHeight.toDouble());

  @override
  void startThread() {
    if (_running) return;
    _running = true;
    _jsStartRenderLoop();
  }

  @override
  void stopThread() {
    _running = false;
    _jsStopRenderLoop();
  }

  @override
  String setShader(
      bool isContinuous, String vertexShader, String fragmentShader) {
    _currentVertexSrc = vertexShader;
    _currentFragmentSrc = fragmentShader;
    return _jsSetShader(
      isContinuous.toJS,
      vertexShader.toJS,
      fragmentShader.toJS,
    ).toDart;
  }

  @override
  String setShaderToy(String fragmentShader) {
    _currentFragmentSrc = fragmentShader;
    _currentVertexSrc = null;
    return _jsSetShaderToy(fragmentShader.toJS).toDart;
  }

  @override
  String getVertexShader() => _currentVertexSrc ?? '';

  @override
  String getFragmentShader() => _currentFragmentSrc ?? '';

  @override
  void addShaderToyUniforms() {
    _jsAddShaderToyUniforms();
  }

  @override
  void setMousePosition(
      Offset startingPos, Offset pos, PointerEventType eventType, Size twSize) {
    if (twSize.width <= 0 || twSize.height <= 0) return;
    final posX = pos.dx;
    final posY = pos.dy;
    final double posZ;
    final double posW;

    if (eventType == PointerEventType.onPointerDown ||
        eventType == PointerEventType.onPointerMove) {
      posZ = startingPos.dx;
      posW = -startingPos.dy;
    } else {
      posZ = -startingPos.dx;
      posW = -startingPos.dy;
    }

    _jsSetMousePosition(
      posX.toJS,
      posY.toJS,
      posZ.toJS,
      posW.toJS,
      twSize.width.toJS,
      twSize.height.toJS,
    );
  }

  @override
  double getFps() => _jsGetFps().toDartDouble;

  // ------------------------------------------------------------------
  // Uniform helpers
  // ------------------------------------------------------------------

  /// Encode a value into a raw byte buffer and call the WASM `add_uniform`.
  bool _addTypedUniform(String name, int typeIndex, Uint8List bytes) {
    return _jsAddUniform(
      name.toJS,
      typeIndex.toJS,
      bytes.toJS,
    ).toDart;
  }

  bool _setTypedUniform(String name, Uint8List bytes) {
    return _jsSetUniform(name.toJS, bytes.toJS).toDart;
  }

  // --- Encode helpers ---

  static Uint8List _encodeBool(bool val) {
    return Uint8List.fromList([val ? 1 : 0]);
  }

  static Uint8List _encodeInt(int val) {
    final bd = ByteData(4);
    bd.setInt32(0, val, Endian.little);
    return bd.buffer.asUint8List();
  }

  static Uint8List _encodeFloat(double val) {
    final bd = ByteData(4);
    bd.setFloat32(0, val, Endian.little);
    return bd.buffer.asUint8List();
  }

  static Uint8List _encodeFloats(List<double> vals) {
    final bd = ByteData(vals.length * 4);
    for (int i = 0; i < vals.length; i++) {
      bd.setFloat32(i * 4, vals[i], Endian.little);
    }
    return bd.buffer.asUint8List();
  }

  // ------------------------------------------------------------------
  // Add uniforms
  // ------------------------------------------------------------------

  @override
  bool addBoolUniform(String name, bool val) =>
      _addTypedUniform(name, 0, _encodeBool(val));

  @override
  bool addIntUniform(String name, int val) =>
      _addTypedUniform(name, 1, _encodeInt(val));

  @override
  bool addFloatUniform(String name, double val) =>
      _addTypedUniform(name, 2, _encodeFloat(val));

  @override
  bool addVec2Uniform(String name, List<double> val) =>
      _addTypedUniform(name, 3, _encodeFloats(val));

  @override
  bool addVec3Uniform(String name, List<double> val) =>
      _addTypedUniform(name, 4, _encodeFloats(val));

  @override
  bool addVec4Uniform(String name, List<double> val) =>
      _addTypedUniform(name, 5, _encodeFloats(val));

  @override
  bool addMat2Uniform(String name, List<double> val) =>
      _addTypedUniform(name, 6, _encodeFloats(val));

  @override
  bool addMat3Uniform(String name, List<double> val) =>
      _addTypedUniform(name, 7, _encodeFloats(val));

  @override
  bool addMat4Uniform(String name, List<double> val) =>
      _addTypedUniform(name, 8, _encodeFloats(val));

  @override
  bool addSampler2DUniform(String name, int width, int height, Uint8List val) {
    return _jsAddSampler2dUniform(
      name.toJS,
      width.toJS,
      height.toJS,
      val.toJS,
    ).toDart;
  }

  // ------------------------------------------------------------------
  // Set uniforms
  // ------------------------------------------------------------------

  @override
  bool setBoolUniform(String name, bool val) =>
      _setTypedUniform(name, _encodeBool(val));

  @override
  bool setIntUniform(String name, int val) =>
      _setTypedUniform(name, _encodeInt(val));

  @override
  bool setFloatUniform(String name, double val) =>
      _setTypedUniform(name, _encodeFloat(val));

  @override
  bool setVec2Uniform(String name, List<double> val) =>
      _setTypedUniform(name, _encodeFloats(val));

  @override
  bool setVec3Uniform(String name, List<double> val) =>
      _setTypedUniform(name, _encodeFloats(val));

  @override
  bool setVec4Uniform(String name, List<double> val) =>
      _setTypedUniform(name, _encodeFloats(val));

  @override
  bool setMat2Uniform(String name, List<double> val) =>
      _setTypedUniform(name, _encodeFloats(val));

  @override
  bool setMat3Uniform(String name, List<double> val) =>
      _setTypedUniform(name, _encodeFloats(val));

  @override
  bool setMat4Uniform(String name, List<double> val) =>
      _setTypedUniform(name, _encodeFloats(val));

  @override
  bool setSampler2DUniform(String name, Uint8List val) =>
      _setTypedUniform(name, val);

  // ------------------------------------------------------------------
  // Replace / remove
  // ------------------------------------------------------------------

  @override
  bool replaceSampler2DUniform(
      String name, int width, int height, Uint8List val) {
    return _jsReplaceSampler2dUniform(
      name.toJS,
      width.toJS,
      height.toJS,
      val.toJS,
    ).toDart;
  }

  @override
  bool removeUniform(String name) {
    return _jsRemoveUniform(name.toJS).toDart;
  }
}
