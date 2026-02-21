import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_vulkan/flutter_vulkan.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../shadertoy.dart';
import '../states.dart';
import 'shader_buttons.dart';
import 'texture_chooser.dart';
import 'texture_sizes.dart';

/// Tab page to test the plugin
/// - create the texture id and use it in the Texture() widget
/// - start/stop renderer
/// - choose shader samples
class Controls extends ConsumerStatefulWidget {
  const Controls({super.key});

  @override
  ConsumerState<Controls> createState() => _ControlsState();
}

class _ControlsState extends ConsumerState<Controls> {
  Timer? fpsTimer;

  @override
  void dispose() {
    fpsTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final textureCreated = ref.watch(stateTextureCreated);

    return SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          /// CREATE TEXTURE
          Wrap(
            alignment: WrapAlignment.center,
            spacing: 8,
            runSpacing: 4,
            children: [
              ElevatedButton(
                style: ButtonStyle(
                  backgroundColor: textureCreated
                      ? const WidgetStatePropertyAll(Colors.green)
                      : const WidgetStatePropertyAll(Colors.red),
                ),
                onPressed: () async {
                  fpsTimer?.cancel();
                  Size textureSize = ref.read(stateTextureSize);
                  int id = await VulkanController().vulkanPlugin.createSurface(
                        textureSize.width.toInt(),
                        textureSize.height.toInt(),
                      );
                  ref.read(stateTextureCreated.notifier).state =
                      VulkanController().renderer.rendererStatus();
                  ref.read(stateTextureId.notifier).state = id;

                  VulkanController().renderer.startThread();

                  int idx = ref.read(stateShaderIndex);
                  if (idx < 0) idx = 0;
                  VulkanController().renderer.setShaderToy(
                      shaderToy[idx]['fragment']!);
                  ref.read(stateUrl.notifier).state = shaderToy[idx]['url']!;
                  ref.read(stateShaderIndex.notifier).state = idx;

                  fpsTimer = Timer.periodic(
                      const Duration(seconds: 1), (timer) {
                    ref.read(stateFPS.notifier).state =
                        VulkanController().renderer.getFps();
                  });
                },
                child: const Text('create texture'),
              ),

              /// START
              ElevatedButton(
                onPressed: () {
                  VulkanController().renderer.startThread();
                  fpsTimer?.cancel();
                  fpsTimer =
                      Timer.periodic(const Duration(seconds: 1), (timer) {
                    double fps = VulkanController().renderer.getFps();
                    ref.read(stateFPS.notifier).state = fps;
                  });
                },
                child: const Text('start'),
              ),
              /// STOP
              ElevatedButton(
                onPressed: () {
                  fpsTimer?.cancel();
                  VulkanController().renderer.stopThread();
                  ref.read(stateFPS.notifier).state = 0;
                },
                child: const Text('stop'),
              ),
            ],
          ),
          const SizedBox(height: 10),

          /// SET TEXTURE SIZE
          const TextureSize(),

          /// SHADERS BUTTONS
          const ShaderButtons(),

          const SizedBox(height: 10),

          /// CHOOSE TEXTURE
          const TextureChooser(),
        ],
      ),
    );
  }
}
