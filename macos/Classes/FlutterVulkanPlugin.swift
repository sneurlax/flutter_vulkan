import FlutterMacOS
import CoreVideo

public class FlutterVulkanPlugin: NSObject, FlutterPlugin {
    private var textureRegistry: FlutterTextureRegistry?
    private var vulkanTexture: VulkanFlutterTexture?
    private var textureId: Int64 = -1
    private var pluginContextPtr: UnsafeMutablePointer<VulkanPluginContext>?

    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(
            name: "flutter_vulkan_plugin",
            binaryMessenger: registrar.messenger
        )
        let instance = FlutterVulkanPlugin()
        instance.textureRegistry = registrar.textures
        registrar.addMethodCallDelegate(instance, channel: channel)
    }

    public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        switch call.method {
        case "createSurface":
            guard let args = call.arguments as? [String: Any],
                  let width = args["width"] as? Int,
                  let height = args["height"] as? Int,
                  width > 0, height > 0 else {
                result(FlutterError(
                    code: "100",
                    message: "createSurface() called without valid width and height parameters!",
                    details: nil
                ))
                return
            }

            // Clean up previous texture if any
            if textureId >= 0 {
                if getRenderer() != nil {
                    stopThread()
                }
                textureRegistry?.unregisterTexture(textureId)
                pluginContextPtr?.deallocate()
                pluginContextPtr = nil
            }

            // Create the Flutter texture
            vulkanTexture = VulkanFlutterTexture(width: width, height: height)
            textureId = textureRegistry!.register(vulkanTexture!)

            // Create the registry info and store as opaque pointer
            let registryInfo = TextureRegistryInfo(
                registry: textureRegistry!,
                textureId: textureId
            )
            vulkanTexture!.registryInfo = registryInfo
            let opaqueRef = Unmanaged.passUnretained(registryInfo).toOpaque()

            // Heap-allocate the plugin context so the pointer remains valid
            // for the C++ render thread's entire lifetime
            pluginContextPtr = .allocate(capacity: 1)
            pluginContextPtr!.initialize(to: VulkanPluginContext(
                buffer: vulkanTexture!.pixelBufferBase,
                width: Int32(width),
                height: Int32(height),
                markFrameAvailable: { registryRef in
                    guard let ref = registryRef else { return }
                    let info = Unmanaged<TextureRegistryInfo>.fromOpaque(ref).takeUnretainedValue()
                    DispatchQueue.main.async {
                        info.registry.textureFrameAvailable(info.textureId)
                    }
                },
                registryRef: UnsafeMutableRawPointer(opaqueRef)
            ))

            createRenderer(pluginContextPtr!)

            result(textureId)

        default:
            result(FlutterMethodNotImplemented)
        }
    }
}

class TextureRegistryInfo {
    let registry: FlutterTextureRegistry
    let textureId: Int64

    init(registry: FlutterTextureRegistry, textureId: Int64) {
        self.registry = registry
        self.textureId = textureId
    }
}
