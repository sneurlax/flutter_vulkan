import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_vulkan/flutter_vulkan.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image/image.dart' as img;

import '../states.dart';

/// Row of 4 TextureWidget that represent the 4 iChannel[0-3]
class TextureChooser extends ConsumerWidget {
  const TextureChooser({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    int shaderIndex = ref.watch(stateShaderIndex);
    if (shaderIndex == -1) {
      return const SizedBox.shrink();
    }

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: List.generate(4, (index) => TextureWidget(channelId: index)),
    );
  }
}

enum AddMethod { add, replace, set }

/// Load an asset image, decode it, and send its RGBA bytes as a sampler2D uniform.
Future<bool> setAssetTexture(
    String uniformName, String assetPath, AddMethod method) async {
  try {
    final data = await rootBundle.load(assetPath);
    final decoded = img.decodeImage(data.buffer.asUint8List());
    if (decoded == null) return false;

    final rgba = decoded.convert(numChannels: 4).getBytes(order: img.ChannelOrder.rgba);

    switch (method) {
      case AddMethod.add:
        return VulkanController().renderer.addSampler2DUniform(
              uniformName, decoded.width, decoded.height, rgba);
      case AddMethod.replace:
        return VulkanController().renderer.replaceSampler2DUniform(
              uniformName, decoded.width, decoded.height, rgba);
      case AddMethod.set:
        return VulkanController()
            .renderer
            .setSampler2DUniform(uniformName, rgba);
    }
  } catch (e) {
    debugPrint('Error loading asset texture: $e');
    return false;
  }
}

/// Widget that displays the current bound texture
class TextureWidget extends ConsumerWidget {
  final int channelId;
  final double? width;
  final double? height;

  const TextureWidget({
    super.key,
    required this.channelId,
    this.width = 80,
    this.height = 80,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final TextureParams texture;
    switch (channelId) {
      case 0:
        texture = ref.watch(stateChannel0);
        break;
      case 1:
        texture = ref.watch(stateChannel1);
        break;
      case 2:
        texture = ref.watch(stateChannel2);
        break;
      case 3:
        texture = ref.watch(stateChannel3);
        break;
      default:
        texture = ref.watch(stateChannel0);
    }

    return Column(
      children: [
        Stack(
          children: [
            /// POPUP MENU for texture selection
            PopupMenuButton<_TextureOption>(
              onSelected: (option) {
                setAssetTexture(
                        'iChannel$channelId', option.assetImage, option.method)
                    .then((value) {
                  if (value) {
                    _setChannelState(ref, option.assetImage);
                  }
                });
              },
              itemBuilder: (_) => [
                for (final item in _textureItems)
                  PopupMenuItem(
                    value: item,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Image.asset(item.assetImage, width: 40, height: 40),
                        const SizedBox(width: 8),
                        Text(item.label),
                        const SizedBox(width: 8),
                        Icon(item.method == AddMethod.add
                            ? Icons.add
                            : item.method == AddMethod.replace
                                ? Icons.find_replace
                                : Icons.settings_overscan_outlined),
                      ],
                    ),
                  ),
              ],
              child: Container(
                width: width,
                height: height,
                margin: const EdgeInsets.all(6),
                decoration: BoxDecoration(
                  color: Colors.black,
                  borderRadius: const BorderRadius.all(Radius.circular(10)),
                  border: Border.all(width: 3, color: Colors.white),
                  image: texture.assetImage.isEmpty
                      ? null
                      : DecorationImage(
                          fit: BoxFit.cover,
                          image: AssetImage(texture.assetImage),
                        ),
                ),
              ),
            ),

            /// REMOVE TEXTURE
            Positioned(
              right: 9,
              top: 9,
              child: GestureDetector(
                onTap: () {
                  bool removed = VulkanController()
                      .renderer
                      .removeUniform('iChannel$channelId');
                  if (removed) {
                    _clearTexture(ref);
                  }
                },
                child: const Icon(Icons.delete_outline, size: 24),
              ),
            ),
          ],
        ),
        Text('iChannel$channelId'),
      ],
    );
  }

  void _clearTexture(WidgetRef ref) {
    switch (channelId) {
      case 0:
        ref.read(stateChannel0.notifier).state =
            TextureParams().copyWith(assetsImage: '');
        break;
      case 1:
        ref.read(stateChannel1.notifier).state =
            TextureParams().copyWith(assetsImage: '');
        break;
      case 2:
        ref.read(stateChannel2.notifier).state =
            TextureParams().copyWith(assetsImage: '');
        break;
      case 3:
        ref.read(stateChannel3.notifier).state =
            TextureParams().copyWith(assetsImage: '');
        break;
    }
  }

  void _setChannelState(WidgetRef ref, String assetImage) {
    switch (channelId) {
      case 0:
        ref.read(stateChannel0.notifier).state =
            TextureParams().copyWith(assetsImage: assetImage);
        break;
      case 1:
        ref.read(stateChannel1.notifier).state =
            TextureParams().copyWith(assetsImage: assetImage);
        break;
      case 2:
        ref.read(stateChannel2.notifier).state =
            TextureParams().copyWith(assetsImage: assetImage);
        break;
      case 3:
        ref.read(stateChannel3.notifier).state =
            TextureParams().copyWith(assetsImage: assetImage);
        break;
    }
  }
}

class _TextureOption {
  final String assetImage;
  final String label;
  final AddMethod method;

  const _TextureOption(this.assetImage, this.label, this.method);
}

const _textureItems = [
  _TextureOption('assets/dash.png', 'dash 1481x900', AddMethod.add),
  _TextureOption('assets/flutter.png', 'flutter 512x512', AddMethod.add),
  _TextureOption(
      'assets/rgba-noise-medium.png', 'noise-med 96x96', AddMethod.add),
  _TextureOption(
      'assets/rgba-noise-small.png', 'noise-sm 96x96', AddMethod.add),
];
