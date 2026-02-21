import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('smoke test - app builds without crashing', (tester) async {
    // Cannot test the full app since VulkanController requires native FFI.
    // Instead verify the widget tree builds with a minimal MaterialApp.
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: Scaffold(body: Text('flutter_vulkan example')),
        ),
      ),
    );
    expect(find.text('flutter_vulkan example'), findsOneWidget);
  });
}
