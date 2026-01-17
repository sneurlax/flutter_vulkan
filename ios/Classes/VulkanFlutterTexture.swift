import Flutter
import CoreVideo

class VulkanFlutterTexture: NSObject, FlutterTexture {
    private var pixelBuffer: CVPixelBuffer?
    let width: Int
    let height: Int
    var pixelBufferBase: UnsafeMutablePointer<UInt8>?
    var registryInfo: TextureRegistryInfo?

    init(width: Int, height: Int) {
        self.width = width
        self.height = height
        super.init()

        // Create a CVPixelBuffer in BGRA format
        let attrs: [String: Any] = [
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferIOSurfacePropertiesKey as String: [:] as [String: Any],
        ]

        let status = CVPixelBufferCreate(
            kCFAllocatorDefault,
            width,
            height,
            kCVPixelFormatType_32BGRA,
            attrs as CFDictionary,
            &pixelBuffer
        )

        if status == kCVReturnSuccess, let pb = pixelBuffer {
            // Lock and get the base address - keep it locked for the render thread to write to
            CVPixelBufferLockBaseAddress(pb, [])
            pixelBufferBase = CVPixelBufferGetBaseAddress(pb)?.assumingMemoryBound(to: UInt8.self)
        }
    }

    deinit {
        if let pb = pixelBuffer {
            CVPixelBufferUnlockBaseAddress(pb, [])
        }
    }

    func copyPixelBuffer() -> Unmanaged<CVPixelBuffer>? {
        guard let pb = pixelBuffer else { return nil }
        return Unmanaged.passRetained(pb)
    }
}
