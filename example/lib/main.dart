import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_vulkan/flutter_vulkan.dart';
import 'package:flutter_vulkan_example/controls/controls.dart';
import 'package:flutter_vulkan_example/edit_shader.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'shadertoy.dart';
import 'states.dart';

void main() {
  VulkanController().initializeVulkan();
  runApp(const ProviderScope(child: MyApp()));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      theme: ThemeData(
        primarySwatch: Colors.blue,
        brightness: Brightness.dark,
      ),
      home: const TextureAndTabs(),
    );
  }
}

class TextureAndTabs extends ConsumerStatefulWidget {
  const TextureAndTabs({super.key});

  @override
  ConsumerState<TextureAndTabs> createState() => _TextureAndTabsState();
}

class _TextureAndTabsState extends ConsumerState<TextureAndTabs> {
  @override
  void initState() {
    super.initState();
    _initRenderer();
  }

  Future<void> _initRenderer() async {
    final size = ref.read(stateTextureSize);
    final id = await VulkanController().vulkanPlugin.createSurface(
          size.width.toInt(),
          size.height.toInt(),
        );
    if (!mounted) return;
    ref.read(stateTextureCreated.notifier).state =
        VulkanController().renderer.rendererStatus();
    ref.read(stateTextureId.notifier).state = id;

    VulkanController().renderer.startThread();

    if (shaderToy.isNotEmpty) {
      VulkanController().renderer.setShaderToy(shaderToy[0]['fragment']!);
      ref.read(stateUrl.notifier).state = shaderToy[0]['url']!;
      ref.read(stateShaderIndex.notifier).state = 0;
    }
  }

  @override
  Widget build(BuildContext context) {
    final textureSize = ref.watch(stateTextureSize);
    final textureId = ref.watch(stateTextureId);

    return DefaultTabController(
      length: 2,
      child: Scaffold(
        body: Padding(
          padding: const EdgeInsets.all(8.0),
          child: Column(
            mainAxisSize: MainAxisSize.max,
            children: [
              const UpperText(),
              const SizedBox(height: 8),

              AspectRatio(
                aspectRatio: textureSize.width / textureSize.height,
                child: textureId == -1
                    ? const ColoredBox(color: Colors.black)
                    : VulkanTexture(id: textureId),
              ),

              const SizedBox(
                height: 40,
                child: TabBar(
                  isScrollable: true,
                  tabs: [
                    Tab(text: 'shaders'),
                    Tab(text: 'edit shader'),
                  ],
                ),
              ),

              const SizedBox(height: 12),

              Expanded(
                child: TabBarView(
                  physics: const NeverScrollableScrollPhysics(),
                  children: [
                    const Controls(),
                    const EditShader(),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// FPS, texture size and shader URL
class UpperText extends ConsumerWidget {
  const UpperText({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fps = ref.watch(stateFPS);
    final shaderUrl = ref.watch(stateUrl);
    final textureSize = ref.watch(stateTextureSize);
    return Wrap(
      alignment: WrapAlignment.center,
      spacing: 30,
      children: [
        Text(
          '${fps.toStringAsFixed(1)} FPS\n'
          '${textureSize.width.toInt()} x '
          '${textureSize.height.toInt()}',
          textAlign: TextAlign.center,
          textScaler: const TextScaler.linear(1.2),
        ),
        if (shaderUrl.isNotEmpty)
          RichText(
            text: TextSpan(
              children: [
                TextSpan(
                  text: shaderUrl,
                  style: const TextStyle(
                    decoration: TextDecoration.underline,
                    fontWeight: FontWeight.bold,
                  ),
                  recognizer: TapGestureRecognizer()..onTap = () {},
                ),
              ],
            ),
          ),
      ],
    );
  }
}
