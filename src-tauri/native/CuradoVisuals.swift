import AppKit
import MetalKit
import simd

struct AudioFrame: Decodable { var rms: Float; var peak: Float; var waveform: [Float]; var accent: [Float]? }
struct Uniforms { var time: Float; var energy: Float; var aspect: Float; var peak: Float; var accent: SIMD4<Float> }
let shader = """
#include <metal_stdlib>
using namespace metal;
struct U { float time; float energy; float aspect; float peak; float4 accent; };
struct V { float4 position [[position]]; float3 color; };
vertex V landscape(uint id [[vertex_id]], constant U& u [[buffer(0)]], constant float* wave [[buffer(1)]]) {
    const uint n = 96;
    uint cell = id / 6; uint corner = id % 6;
    uint2 offsets[6] = { uint2(0,0), uint2(1,0), uint2(0,1), uint2(1,0), uint2(1,1), uint2(0,1) };
    uint2 grid = uint2(cell % n, cell / n) + offsets[corner];
    float x = (float(grid.x) / n - 0.5) * 16.;
    float z = float(grid.y) / n * 25.;
    float a = sin(x * 0.7 + z * 0.48 + u.time * 0.6);
    float b = cos(z * 0.8 - u.time * 0.4 + x * 0.31);
    float h = (a * b) * (0.4 + u.energy * 6.) + abs(wave[grid.x % 64]) * 3.;
    float depth = 3. + z;
    V o; o.position = float4(x / (depth * 0.55 * u.aspect), (h - 1.7) / (depth * 0.5) + 0.3, z / 30., 1.);
    float edge = (grid.x % 4 == 0 || grid.y % 4 == 0) ? 1.0 : 0.22;
    o.color = mix(u.accent.rgb * 0.23, u.accent.rgb, clamp((h+1.)*0.4,0.,1.)) * edge * (1. - z/35.);
    return o;
}
fragment float4 shade(V v [[stage_in]]) { return float4(v.color, 1.); }
"""
final class Visualizer: NSObject, NSApplicationDelegate, MTKViewDelegate {
    var window: NSWindow!
    var view: MTKView!
    var queue: MTLCommandQueue!
    var pipeline: MTLRenderPipelineState!
    var energy: Float = 0
    var peak: Float = 0
    var accent = SIMD4<Float>(214.0/255.0, 97.0/255.0, 84.0/255.0, 1)
    var waveform = [Float](repeating: 0, count: 64)
    let started = ProcessInfo.processInfo.systemUptime
    func applicationDidFinishLaunching(_ notification: Notification) {
        guard let device = MTLCreateSystemDefaultDevice() else { NSApp.terminate(nil); return }
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1100, height: 720), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
        window.title = "Curado · Paisaje sonoro / Metal"
        view = MTKView(frame: window.contentView!.bounds, device: device)
        view.clearColor = MTLClearColorMake(0.025, 0.04, 0.04, 1)
        view.preferredFramesPerSecond = 30
        view.autoresizingMask = [.width, .height]
        queue = device.makeCommandQueue()
        do {
            let library = try device.makeLibrary(source: shader, options: nil)
            let descriptor = MTLRenderPipelineDescriptor()
            descriptor.vertexFunction = library.makeFunction(name: "landscape")
            descriptor.fragmentFunction = library.makeFunction(name: "shade")
            descriptor.colorAttachments[0].pixelFormat = view.colorPixelFormat
            pipeline = try device.makeRenderPipelineState(descriptor: descriptor)
        } catch { fputs("Metal: \(error)\n", stderr); NSApp.terminate(nil); return }
        view.delegate = self
        window.contentView!.addSubview(view)
        window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        DispatchQueue.global(qos: .utility).async { [weak self] in
            while let line = readLine() {
                guard let data = line.data(using: .utf8), let frame = try? JSONDecoder().decode(AudioFrame.self, from: data) else { continue }
                DispatchQueue.main.async { if let color = frame.accent, color.count == 3 { self?.accent = SIMD4<Float>(color[0],color[1],color[2],1) }; self?.energy = frame.rms; self?.peak = frame.peak; self?.waveform = Array((frame.waveform + [Float](repeating: 0, count: 64)).prefix(64)) }
            }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
    }
    func draw(in view: MTKView) {
        guard window.isVisible, !window.isMiniaturized, let drawable = view.currentDrawable, let pass = view.currentRenderPassDescriptor, let buffer = queue.makeCommandBuffer(), let encoder = buffer.makeRenderCommandEncoder(descriptor: pass) else { return }
        var u = Uniforms(time: Float(ProcessInfo.processInfo.systemUptime - started), energy: energy, aspect: Float(view.drawableSize.width / max(1, view.drawableSize.height)), peak: peak, accent: accent)
        encoder.setRenderPipelineState(pipeline)
        encoder.setVertexBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        waveform.withUnsafeBytes { bytes in encoder.setVertexBytes(bytes.baseAddress!, length: bytes.count, index: 1) }
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 96 * 96 * 6)
        encoder.endEncoding(); buffer.present(drawable); buffer.commit()
    }
    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}
let app = NSApplication.shared
let delegate = Visualizer()
app.setActivationPolicy(.regular)
app.delegate = delegate
app.run()
