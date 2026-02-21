import 'package:flutter/material.dart';
import 'package:flutter_vulkan/flutter_vulkan.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../shadertoy.dart';
import '../states.dart';
import 'texture_chooser.dart';

/// Shader buttons (without texture)
class ShaderButtons extends ConsumerWidget {
  const ShaderButtons({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeButtonId = ref.watch(stateShaderIndex);
    final channels = [stateChannel0, stateChannel1, stateChannel2, stateChannel3];

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text('Shader examples'),
        Wrap(
          alignment: WrapAlignment.center,
          runSpacing: 4,
          spacing: 4,
          children: [
            ...List.generate(shaderToy.length, (i) {
              bool hasIChannel0 =
                  shaderToy[i]['fragment']!.contains('iChannel0');
              bool hasIChannel1 =
                  shaderToy[i]['fragment']!.contains('iChannel1');
              bool hasIChannel2 =
                  shaderToy[i]['fragment']!.contains('iChannel2');
              bool hasIChannel3 =
                  shaderToy[i]['fragment']!.contains('iChannel3');
              return ElevatedButton(
                onPressed: () async {
                  ref.read(stateUrl.notifier).state = shaderToy[i]['url']!;
                  VulkanController().renderer.setShaderToy(
                        shaderToy[i]['fragment']!,
                      );
                  ref.read(stateShaderIndex.notifier).state = i;

                  // Auto-load a default texture for each iChannel the
                  // shader uses so it doesn't render black.
                  const defaultAsset = 'assets/flutter.png';
                  final frag = shaderToy[i]['fragment']!;
                  for (int ch = 0; ch < 4; ch++) {
                    if (frag.contains('iChannel$ch')) {
                      await setAssetTexture(
                          'iChannel$ch', defaultAsset, AddMethod.replace);
                      ref.read(channels[ch].notifier).state =
                          TextureParams().copyWith(assetsImage: defaultAsset);
                    } else {
                      ref.read(channels[ch].notifier).state =
                          TextureParams().copyWith(assetsImage: '');
                    }
                  }
                },
                style: ButtonStyle(
                  fixedSize: const WidgetStatePropertyAll(Size(65, 55)),
                  padding: const WidgetStatePropertyAll(
                      EdgeInsets.symmetric(horizontal: 4, vertical: 2)),
                  backgroundColor: i == activeButtonId
                      ? const WidgetStatePropertyAll(Colors.green)
                      : null,
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text('${i + 1}'),
                    Wrap(
                      children: [
                        if (hasIChannel0)
                          const Text('0 ',
                              textScaler: TextScaler.linear(0.8)),
                        if (hasIChannel1)
                          const Text('1 ',
                              textScaler: TextScaler.linear(0.8)),
                        if (hasIChannel2)
                          const Text('2 ',
                              textScaler: TextScaler.linear(0.8)),
                        if (hasIChannel3)
                          const Text('3',
                              textScaler: TextScaler.linear(0.8)),
                      ],
                    ),
                  ],
                ),
              );
            }),
          ],
        ),
      ],
    );
  }
}
