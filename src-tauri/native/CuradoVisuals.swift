import AppKit
import MetalKit
import simd

struct AudioFrame: Decodable { var rms: Float; var peak: Float; var waveform: [Float]; var accent: [Float]? }
struct Uniforms { var time: Float; var energy: Float; var aspect: Float; var peak: Float; var accent: SIMD4<Float>; var settings: SIMD4<Float> }
let shader = """
#include <metal_stdlib>
using namespace metal;
struct U { float time; float energy; float aspect; float peak; float4 accent; float4 settings; };
struct V { float4 position [[position]]; float3 color; };
struct Screen { float4 position [[position]]; float2 uv; };
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
vertex Screen screen(uint id [[vertex_id]]) {
    float2 points[3] = { float2(-1., -1.), float2(3., -1.), float2(-1., 3.) };
    Screen o; o.position = float4(points[id], 0., 1.); o.uv = points[id]; return o;
}
float lineGlow(float distance, float width, float glow) {
    return (1. - smoothstep(width, width * 2., distance)) + glow * 0.12 * exp(-distance * 18.);
}
float3 palette(float3 accent, float phase) {
    float3 shifted = accent.gbr * 0.65 + float3(0.12, 0.08, 0.25);
    return mix(accent, shifted, 0.5 + 0.5 * sin(phase));
}
fragment float4 scenes(Screen in [[stage_in]], constant U& u [[buffer(0)]], constant float* wave [[buffer(1)]]) {
    float2 p = in.uv * float2(u.aspect, 1.);
    float r = length(p);
    float angle = atan2(p.y, p.x);
    float energy = clamp(u.energy, 0., 1.);
    float glow = u.settings.y;
    float3 color = float3(0.012, 0.014, 0.035) + u.accent.rgb * 0.025 * (1. - min(r, 1.));
    int mode = int(u.settings.x);
    if (mode == 1) {
        // A shaded sphere with rotating latitude/longitude lines and audio-driven orbit rings.
        float radius = 0.42 + energy * 0.13;
        float d = r / radius;
        if (d < 1.) {
            float z = sqrt(max(0., 1. - d*d));
            float longitude = atan2(p.x / radius, z) + u.time * 0.32;
            float latitude = asin(clamp(p.y / radius, -1., 1.));
            float grid = max(pow(abs(sin(longitude * 8.)), 32.), pow(abs(sin(latitude * 10.)), 32.));
            color += palette(u.accent.rgb, latitude * 4. + u.time * 0.3) * (0.12 + 0.35*z + grid * 0.75);
        }
        color += palette(u.accent.rgb, angle + u.time * 0.3) * glow * 0.28 * exp(-abs(r-radius)*12.);
        for (int i = 0; i < 3; i++) {
            float2 q = float2(p.x, p.y * (1.8 + float(i)*0.45) + sin(p.x*3. + u.time + float(i))*0.06);
            float orbit = abs(length(q) - radius*(1.28 + float(i)*0.22));
            color += palette(u.accent.rgb, float(i)*2. + u.time*0.4) * lineGlow(orbit, 0.0025, glow) * 0.5;
        }
    } else if (mode == 2) {
        // Polar tunnel: depth and color cycle independently of the audio clock.
        float depth = 1. / max(r, 0.08);
        float rings = abs(sin(depth * 2.5 - u.time * (1.1 + energy)));
        float spokes = abs(sin(angle * 8. + depth * 0.65 + u.time * 0.2));
        float grid = pow(rings, 32.) * 0.65 + pow(spokes, 32.) * 0.45;
        float fade = smoothstep(0.06, 0.24, r) * (1. - smoothstep(1.0, 1.8, r));
        color += palette(u.accent.rgb, depth * 0.7 - u.time * 0.4) * (grid + glow * 0.14 * (rings + spokes)) * fade * (0.6 + energy);
    } else {
        // Wave samples bend multiple luminous ribbons; these are not frequency bands.
        float x = clamp((in.uv.x + 1.) * 0.5, 0., 0.999);
        uint index = min(uint(x * 64.), 63u);
        float sample = clamp(wave[index], -1., 1.);
        for (int i = 0; i < 7; i++) {
            float fi = float(i);
            float y = sin(p.x * (2.2 + fi * 0.25) + u.time * (0.55 + fi * 0.08) + fi) * (0.10 + energy * 0.25);
            y += sample * (0.18 + fi * 0.035) + (fi - 3.) * 0.11;
            float distance = abs(p.y - y);
            color += palette(u.accent.rgb, fi * 0.9 + p.x * 1.5 + u.time * 0.25) * lineGlow(distance, 0.003, glow) * 0.42;
        }
    }
    return float4(min(color, float3(1.)), 1.);
}
"""
final class Visualizer: NSObject, NSApplicationDelegate, MTKViewDelegate {
    var window: NSWindow!
    var view: MTKView!
    var queue: MTLCommandQueue!
    var landscapePipeline: MTLRenderPipelineState!
    var scenePipeline: MTLRenderPipelineState!
    var scene = 0
    var glow: Float = 1
    var energy: Float = 0
    var peak: Float = 0
    var accent = SIMD4<Float>(214.0/255.0, 97.0/255.0, 84.0/255.0, 1)
    var waveform = [Float](repeating: 0, count: 64)
    let started = ProcessInfo.processInfo.systemUptime
    func applicationDidFinishLaunching(_ notification: Notification) {
        guard let device = MTLCreateSystemDefaultDevice(), let commandQueue = device.makeCommandQueue() else { NSApp.terminate(nil); return }
        queue = commandQueue
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1100, height: 720), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
        window.title = "curado · paisaje sonoro"
        window.minSize = NSSize(width: 440, height: 260)
        view = MTKView(frame: .zero, device: device)
        view.clearColor = MTLClearColorMake(0.025, 0.04, 0.04, 1)
        view.preferredFramesPerSecond = 30
        do {
            let library = try device.makeLibrary(source: shader, options: nil)
            func makePipeline(_ vertex: String, _ fragment: String) throws -> MTLRenderPipelineState {
                let descriptor = MTLRenderPipelineDescriptor()
                descriptor.vertexFunction = library.makeFunction(name: vertex)
                descriptor.fragmentFunction = library.makeFunction(name: fragment)
                descriptor.colorAttachments[0].pixelFormat = view.colorPixelFormat
                return try device.makeRenderPipelineState(descriptor: descriptor)
            }
            landscapePipeline = try makePipeline("landscape", "shade")
            scenePipeline = try makePipeline("screen", "scenes")
        } catch { fputs("Metal: \(error)\n", stderr); NSApp.terminate(nil); return }
        view.delegate = self
        let scenePicker = NSPopUpButton()
        scenePicker.addItems(withTitles: ["Terreno", "Órbita", "Túnel", "Cintas"])
        scenePicker.target = self; scenePicker.action = #selector(changeScene(_:))
        scenePicker.toolTip = "Elegir visual"
        scenePicker.setAccessibilityLabel("Elegir visual")
        let glowSlider = NSSlider(value: 1, minValue: 0, maxValue: 2, target: self, action: #selector(changeGlow(_:)))
        glowSlider.toolTip = "Intensidad del brillo en Órbita, Túnel y Cintas"
        glowSlider.setAccessibilityLabel("Intensidad del brillo")
        glowSlider.widthAnchor.constraint(equalToConstant: 120).isActive = true
        let controls = NSStackView(views: [NSTextField(labelWithString: "Visual"), scenePicker, NSTextField(labelWithString: "Brillo"), glowSlider])
        controls.orientation = .horizontal; controls.spacing = 10; controls.alignment = .centerY
        view.translatesAutoresizingMaskIntoConstraints = false
        controls.translatesAutoresizingMaskIntoConstraints = false
        let content = window.contentView!
        content.addSubview(view); content.addSubview(controls)
        NSLayoutConstraint.activate([
            controls.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 16),
            controls.topAnchor.constraint(equalTo: content.topAnchor, constant: 8),
            controls.heightAnchor.constraint(equalToConstant: 28),
            view.topAnchor.constraint(equalTo: controls.bottomAnchor, constant: 8),
            view.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            view.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            view.bottomAnchor.constraint(equalTo: content.bottomAnchor)
        ])
        window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        DispatchQueue.global(qos: .utility).async { [weak self] in
            while let line = readLine() {
                guard let data = line.data(using: .utf8), let frame = try? JSONDecoder().decode(AudioFrame.self, from: data) else { continue }
                DispatchQueue.main.async { if let color = frame.accent, color.count == 3 { self?.accent = SIMD4<Float>(color[0],color[1],color[2],1) }; self?.energy = frame.rms; self?.peak = frame.peak; self?.waveform = Array((frame.waveform + [Float](repeating: 0, count: 64)).prefix(64)) }
            }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
    }
    @objc func changeScene(_ sender: NSPopUpButton) { scene = sender.indexOfSelectedItem }
    @objc func changeGlow(_ sender: NSSlider) { glow = sender.floatValue }
    func draw(in view: MTKView) {
        guard window.isVisible, !window.isMiniaturized, let drawable = view.currentDrawable, let pass = view.currentRenderPassDescriptor, let buffer = queue.makeCommandBuffer(), let encoder = buffer.makeRenderCommandEncoder(descriptor: pass) else { return }
        var u = Uniforms(time: Float(ProcessInfo.processInfo.systemUptime - started), energy: energy, aspect: Float(view.drawableSize.width / max(1, view.drawableSize.height)), peak: peak, accent: accent, settings: SIMD4<Float>(Float(scene), glow, 0, 0))
        encoder.setRenderPipelineState(scene == 0 ? landscapePipeline : scenePipeline)
        encoder.setVertexBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        encoder.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        waveform.withUnsafeBytes { bytes in
            encoder.setVertexBytes(bytes.baseAddress!, length: bytes.count, index: 1)
            encoder.setFragmentBytes(bytes.baseAddress!, length: bytes.count, index: 1)
        }
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: scene == 0 ? 96 * 96 * 6 : 3)
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
