import FlutterMacOS
import CoreVideo

// C-compatible frame-available callback (no captures, so usable as @convention(c)).
private func frameAvailableCallback(_ userData: UnsafeMutableRawPointer?) {
    guard let ref = userData else { return }
    let info = Unmanaged<TextureRegistryInfo>.fromOpaque(ref).takeUnretainedValue()
    DispatchQueue.main.async {
        info.registry.textureFrameAvailable(info.textureId)
    }
}

public class FlutterVulkanPlugin: NSObject, FlutterPlugin {
    private var textureRegistry: FlutterTextureRegistry?
    private var vulkanTexture: VulkanFlutterTexture?
    private var textureId: Int64 = -1
    private var registryInfo: TextureRegistryInfo?

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
                registryInfo = nil
            }

            // Create the Flutter texture
            vulkanTexture = VulkanFlutterTexture(width: width, height: height)
            textureId = textureRegistry!.register(vulkanTexture!)

            // Keep registryInfo alive for the render thread's callback lifetime
            let info = TextureRegistryInfo(
                registry: textureRegistry!,
                textureId: textureId
            )
            vulkanTexture!.registryInfo = info
            registryInfo = info
            let opaqueRef = Unmanaged.passUnretained(info).toOpaque()

            createRenderer(vulkanTexture!.pixelBufferBase, Int32(width), Int32(height))
            setFrameCallback(frameAvailableCallback, opaqueRef)

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
