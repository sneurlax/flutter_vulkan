import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

final stateFPS = StateProvider<double>((ref) => 0.0);

final stateUrl = StateProvider<String>((ref) => '');

final stateTextureCreated = StateProvider<bool>((ref) => false);
final stateTextureSize = StateProvider<Size>((ref) => const Size(600, 337));
final stateTextureId = StateProvider<int>((ref) => -1);

/// current index in the shaderToy list
final stateShaderIndex = StateProvider<int>((ref) => -1);

class TextureParams {
  final String assetImage;

  TextureParams({this.assetImage = ''});

  TextureParams copyWith({String? assetsImage}) {
    return TextureParams(assetImage: assetsImage ?? assetImage);
  }
}

final stateChannel0 = StateProvider<TextureParams>((ref) => TextureParams());
final stateChannel1 = StateProvider<TextureParams>((ref) => TextureParams());
final stateChannel2 = StateProvider<TextureParams>((ref) => TextureParams());
final stateChannel3 = StateProvider<TextureParams>((ref) => TextureParams());
