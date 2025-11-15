import 'package:flutter/material.dart';
import 'package:flutter_vulkan/src/vulkan_controller.dart';

import 'flutter_vulkan_ffi.dart';

class VulkanTexture extends StatelessWidget {
  final int id;

  const VulkanTexture({
    super.key,
    required this.id,
  });

  @override
  Widget build(BuildContext context) {
    Size twSize = Size.zero;
    Offset startingPos = Offset.zero;
    var key = GlobalKey();

    return Listener(
      onPointerDown: (event) {
        startingPos = event.localPosition;
        VulkanController().vulkanFFI.setMousePosition(
              startingPos,
              event.localPosition,
              PointerEventType.onPointerDown,
              twSize,
            );
      },
      onPointerMove: (event) {
        VulkanController().vulkanFFI.setMousePosition(
              startingPos,
              event.localPosition,
              PointerEventType.onPointerMove,
              twSize,
            );
      },
      onPointerUp: (event) {
        VulkanController().vulkanFFI.setMousePosition(
              startingPos,
              event.localPosition,
              PointerEventType.onPointerUp,
              twSize,
            );
      },
      child: LayoutBuilder(builder: (_, __) {
        WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
          final box = context.findRenderObject() as RenderBox;
          twSize = box.size;
        });

        return ColoredBox(
          key: key,
          color: Colors.black,
          child: Texture(textureId: id),
        );
      }),
    );
  }
}
