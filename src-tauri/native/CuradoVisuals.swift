import AppKit
import MetalKit
import simd

struct AudioFrame: Decodable { var rms: Float; var peak: Float; var waveform: [Float]; var accent: [Float]?; var spectrum: [Float]? }
struct Uniforms { var time: Float; var energy: Float; var aspect: Float; var peak: Float; var accent: SIMD4<Float>; var settings: SIMD4<Float>; var camera: SIMD4<Float>; var bands: SIMD4<Float> }
let shader = """
#include <metal_stdlib>
using namespace metal;
struct U { float time; float energy; float aspect; float peak; float4 accent; float4 settings; float4 camera; float4 bands; };
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
    float h = (a * b) * (0.4 + u.bands.x * 3.) + abs(wave[grid.x % 64]) * 3. * u.settings.z;
    float depth = 3. + z;
    V o; o.position = float4((x + u.camera.x * z * 0.18) / (depth * 0.55 * u.aspect), (h - 1.7) / (depth * 0.5) + 0.3 + u.camera.y * 0.08, z / 30., 1.);
    float edge = (grid.x % 4 == 0 || grid.y % 4 == 0) ? 1.0 : 0.22;
    float3 tint = mix(u.accent.rgb, u.accent.gbr, 0.35 * (0.5 + 0.5*sin(u.settings.w)));
    o.color = mix(tint * 0.23, tint, clamp((h+1.)*0.4,0.,1.)) * edge * (1. - z/35.);
    return o;
}
fragment float4 shade(V v [[stage_in]]) { return float4(v.color, 1.); }
vertex V city(uint id [[vertex_id]], constant U& u [[buffer(0)]], constant float* spectrum [[buffer(2)]]) {
    // 12 x 12 low-poly towers; each column follows a logarithmic frequency bin.
    uint cube = id / 36, vertexIndex = id % 36;
    const uint indices[36] = {0,2,1, 1,2,3, 4,5,6, 5,7,6, 0,1,4, 1,5,4, 2,6,3, 3,6,7, 0,4,2, 2,4,6, 1,3,5, 3,7,5};
    uint corner = indices[vertexIndex];
    uint x = cube % 12, z = cube / 12;
    uint bin = min(x * 2 + (z % 2), 23u);
    float level = clamp(spectrum[bin] * u.settings.z, 0., 2.);
    float h = 0.12 + level * 3.5;
    float3 pos = float3((float(x)-5.5)*1.1 + ((corner & 1u) != 0 ? 0.42 : -0.42),
                        -2.0 + ((corner & 2u) != 0 ? h : 0.),
                        (float(z)-5.5)*1.1 + ((corner & 4u) != 0 ? 0.42 : -0.42));
    float yaw = u.time * 0.12 + u.camera.x;
    float cs = cos(yaw), sn = sin(yaw);
    float2 rotated = float2(pos.x*cs - pos.z*sn, pos.x*sn + pos.z*cs);
    float depth = rotated.y + 17.;
    float y = pos.y + u.camera.y * rotated.y * 0.45;
    V o; o.position = float4(rotated.x / u.aspect, y, depth * 0.99 - 0.2, depth);
    float face = vertexIndex / 6 == 1 ? 1.0 : (vertexIndex / 6 == 0 ? 0.32 : 0.58);
    float phase = float(bin) / 24. * 5. + u.settings.w;
    float3 shifted = u.accent.gbr * 0.65 + float3(0.12, 0.08, 0.25);
    o.color = mix(u.accent.rgb, shifted, 0.5 + 0.5*sin(phase)) * (0.18 + level * 0.65) * face;
    return o;
}
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
fragment float4 scenes(Screen in [[stage_in]], constant U& u [[buffer(0)]], constant float* wave [[buffer(1)]], constant float* spectrum [[buffer(2)]]) {
    float2 p = in.uv * float2(u.aspect, 1.);
    p += u.camera.xy * 0.12;
    float r = length(p);
    float angle = atan2(p.y, p.x);
    float energy = clamp(u.energy * u.settings.z, 0., 1.);
    float bass = u.bands.x, mid = u.bands.y, treble = u.bands.z;
    float glow = u.settings.y;
    float3 color = float3(0.012, 0.014, 0.035) + u.accent.rgb * 0.025 * (1. - min(r, 1.));
    int mode = int(u.settings.x);
    if (mode == 1) {
        // A shaded sphere with rotating latitude/longitude lines and audio-driven orbit rings.
        float radius = 0.39 + bass * 0.16;
        float d = r / radius;
        if (d < 1.) {
            float z = sqrt(max(0., 1. - d*d));
            float longitude = atan2(p.x / radius, z) + u.time * 0.32;
            float latitude = asin(clamp(p.y / radius, -1., 1.));
            float grid = max(pow(abs(sin(longitude * 8.)), 32.), pow(abs(sin(latitude * 10.)), 32.));
            color += palette(u.accent.rgb, u.settings.w + latitude * 4. + u.time * 0.3) * (0.12 + 0.35*z + grid * 0.75);
        }
        color += palette(u.accent.rgb, u.settings.w + angle + u.time * 0.3) * glow * 0.28 * exp(-abs(r-radius)*12.);
        for (int i = 0; i < 3; i++) {
            float2 q = float2(p.x, p.y * (1.8 + float(i)*0.45) + sin(p.x*3. + u.time + float(i))*0.06);
            float orbit = abs(length(q) - radius*(1.28 + float(i)*0.22));
            color += palette(u.accent.rgb, u.settings.w + float(i)*2. + u.time*0.4) * lineGlow(orbit, 0.0025, glow) * 0.5;
        }
    } else if (mode == 2) {
        // Polar tunnel: depth and color cycle independently of the audio clock.
        float depth = 1. / max(r, 0.08);
        float rings = abs(sin(depth * (2.3 + bass * 0.5) - u.time * (1.1 + energy)));
        float spokes = abs(sin(angle * (8. + treble * 4.) + depth * 0.65 + u.time * 0.2));
        float grid = pow(rings, 32.) * 0.65 + pow(spokes, 32.) * 0.45;
        float fade = smoothstep(0.06, 0.24, r) * (1. - smoothstep(1.0, 1.8, r));
        color += palette(u.accent.rgb, u.settings.w + depth * 0.7 - u.time * 0.4) * (grid + glow * 0.14 * (rings + spokes)) * fade * (0.6 + energy);
    } else if (mode == 3) {
        // Wave samples bend multiple luminous ribbons; these are not frequency bands.
        float x = clamp((in.uv.x + 1.) * 0.5, 0., 0.999);
        uint index = min(uint(x * 64.), 63u);
        float sample = clamp(wave[index], -1., 1.);
        for (int i = 0; i < 7; i++) {
            float fi = float(i);
            float y = sin(p.x * (2.2 + fi * 0.25) + u.time * (0.55 + fi * 0.08) + fi) * (0.10 + energy * 0.25);
            y += sample * (0.18 + fi * 0.035) + (fi - 3.) * 0.11;
            float distance = abs(p.y - y);
            color += palette(u.accent.rgb, u.settings.w + fi * 0.9 + p.x * 1.5 + u.time * 0.25) * lineGlow(distance, 0.003, glow) * 0.42;
        }
    } else if (mode == 4) {
        // Faceted, recursive gateway with depth layers and drifting points.
        float a = angle + u.time * 0.18 + u.camera.x;
        for (int i = 0; i < 8; i++) {
            float layer = fract(float(i)/8. + u.time*0.08);
            float radius = 0.08 + layer*1.35;
            float facets = cos(floor((a + 3.14159) / 0.7854) * 0.7854 - a) * r;
            float d = abs(facets - radius * (1. + bass*0.12));
            float fade = (1. - layer) * smoothstep(0., 0.15, layer);
            color += palette(u.accent.rgb, u.settings.w + float(i)*0.8 + u.settings.w) * lineGlow(d, 0.003 + layer*0.003, glow) * fade * (0.4 + mid);
        }
        float2 cells = floor((p + u.time*float2(0.04, -0.025))*30.);
        float star = fract(sin(dot(cells, float2(127.1,311.7)))*43758.5453);
        float2 point = fract((p + u.time*float2(0.04, -0.025))*30.) - 0.5;
        color += palette(u.accent.rgb, u.settings.w + star*6. + u.settings.w) * (star > 0.97 ? 1. : 0.) * exp(-length(point)*18.) * (0.3 + treble*1.8);
    } else {
        // Circular oscilloscope and 24 spectral bars: independent FFT bins rise only when active.
        float polar = fract((angle + 3.14159) / 6.28318);
        uint wi = min(uint(polar*64.), 63u);
        uint bi = min(uint(polar*24.), 23u);
        float waveRadius = 0.36 + clamp(wave[wi], -1., 1.) * 0.22 * u.settings.z;
        color += palette(u.accent.rgb, u.settings.w + angle + u.settings.w) * lineGlow(abs(r-waveRadius), 0.004, glow);
        float bin = clamp(spectrum[bi] * u.settings.z, 0., 1.);
        float bar = smoothstep(0.14, 0.85, abs(sin(polar*24.*3.14159)));
        float lengthMask = smoothstep(0.31, 0.33, r) * (1. - smoothstep(0.34 + bin*0.55, 0.35 + bin*0.55, r));
        color += palette(u.accent.rgb, u.settings.w + float(bi)*0.25 + u.settings.w) * bar * lengthMask * (0.25 + bin);
    }
    return float4(min(color, float3(1.)), 1.);
}
"""
final class VisualWindow: NSWindow {
    var onShortcut: ((String) -> Bool)?
    override func sendEvent(_ event: NSEvent) {
        if event.type == .keyDown && event.modifierFlags.intersection(.deviceIndependentFlagsMask).isEmpty,
           let key = event.charactersIgnoringModifiers?.lowercased(), onShortcut?(key) == true { return }
        super.sendEvent(event)
    }
}
final class InteractiveMetalView: MTKView {
    var onDrag: ((NSPoint, Bool) -> Void)?
    override var acceptsFirstResponder: Bool { true }
    override func mouseDown(with event: NSEvent) { onDrag?(convert(event.locationInWindow, from: nil), true) }
    override func mouseDragged(with event: NSEvent) { onDrag?(convert(event.locationInWindow, from: nil), true) }
    override func mouseUp(with event: NSEvent) { onDrag?(.zero, false) }
}
final class Visualizer: NSObject, NSApplicationDelegate, MTKViewDelegate {
    var window: VisualWindow!
    var view: InteractiveMetalView!
    var controls: NSVisualEffectView!
    var toolbar: NSStackView!
    var scenePicker: NSPopUpButton!
    var queue: MTLCommandQueue!
    var landscapePipeline: MTLRenderPipelineState!
    var scenePipeline: MTLRenderPipelineState!
    var cityPipeline: MTLRenderPipelineState!
    var depthState: MTLDepthStencilState!
    var scene = 0
    var glow: Float = 1
    var sensitivity: Float = 1
    var speed: Float = 1
    var hue: Float = 0
    var energy: Float = 0
    var peak: Float = 0
    var accent = SIMD4<Float>(214.0/255.0, 97.0/255.0, 84.0/255.0, 1)
    var waveform = [Float](repeating: 0, count: 64)
    var spectrum = [Float](repeating: 0, count: 24)
    var camera = SIMD2<Float>(0, 0)
    var cameraTarget = SIMD2<Float>(0, 0)
    var velocity = SIMD2<Float>(0, 0)
    var animationTime: Float = 0
    var lastFrame = ProcessInfo.processInfo.systemUptime
    func applicationDidFinishLaunching(_ notification: Notification) {
        guard let device = MTLCreateSystemDefaultDevice(), let commandQueue = device.makeCommandQueue() else { NSApp.terminate(nil); return }
        queue = commandQueue
        window = VisualWindow(contentRect: NSRect(x: 0, y: 0, width: 1100, height: 720), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
        window.title = "curado · paisaje sonoro"
        window.minSize = NSSize(width: 480, height: 340)
        window.collectionBehavior.insert(.fullScreenPrimary)
        window.onShortcut = { [weak self] key in
            guard let self = self else { return false }
            switch key {
            case "h": self.toggleControls(nil)
            case "f": self.toggleFullscreen(nil)
            case "r": self.resetCamera(nil)
            default: return false
            }
            return true
        }
        view = InteractiveMetalView(frame: .zero, device: device)
        view.clearColor = MTLClearColorMake(0.012, 0.014, 0.035, 1)
        view.depthStencilPixelFormat = .depth32Float
        view.preferredFramesPerSecond = 30
        view.onDrag = { [weak self] point, active in
            guard let self = self else { return }
            if active {
                self.cameraTarget = SIMD2<Float>(Float(point.x / max(1, self.view.bounds.width) - 0.5) * 2, Float(point.y / max(1, self.view.bounds.height) - 0.5) * 1.2)
            } else { self.cameraTarget = .zero }
        }
        do {
            let library = try device.makeLibrary(source: shader, options: nil)
            func makePipeline(_ vertex: String, _ fragment: String) throws -> MTLRenderPipelineState {
                let descriptor = MTLRenderPipelineDescriptor()
                descriptor.vertexFunction = library.makeFunction(name: vertex)
                descriptor.fragmentFunction = library.makeFunction(name: fragment)
                descriptor.colorAttachments[0].pixelFormat = view.colorPixelFormat
                descriptor.depthAttachmentPixelFormat = view.depthStencilPixelFormat
                return try device.makeRenderPipelineState(descriptor: descriptor)
            }
            landscapePipeline = try makePipeline("landscape", "shade")
            scenePipeline = try makePipeline("screen", "scenes")
            cityPipeline = try makePipeline("city", "shade")
            let depth = MTLDepthStencilDescriptor()
            depth.depthCompareFunction = .less
            depth.isDepthWriteEnabled = true
            depthState = device.makeDepthStencilState(descriptor: depth)
        } catch { fputs("Metal: \(error)\n", stderr); NSApp.terminate(nil); return }
        view.delegate = self
        scenePicker = NSPopUpButton()
        scenePicker.addItems(withTitles: ["Terreno", "Órbita", "Túnel", "Cintas", "Umbral", "Osciloscopio", "Ciudad espectral"])
        scenePicker.target = self; scenePicker.action = #selector(changeScene(_:))
        scenePicker.setAccessibilityLabel("Elegir visual")
        func slider(_ label: String, _ value: Double, _ max: Double, _ action: Selector) -> NSStackView {
            let title = NSTextField(labelWithString: label)
            title.widthAnchor.constraint(equalToConstant: 92).isActive = true
            let control = NSSlider(value: value, minValue: 0, maxValue: max, target: self, action: action)
            control.setAccessibilityLabel(label)
            control.widthAnchor.constraint(equalToConstant: 140).isActive = true
            let row = NSStackView(views: [title, control]); row.orientation = .horizontal; row.spacing = 8
            return row
        }
        let sceneRow = NSStackView(views: [NSTextField(labelWithString: "Escena"), scenePicker])
        sceneRow.orientation = .horizontal; sceneRow.spacing = 8
        let hint = NSTextField(wrappingLabelWithString: "Arrastra para mover la cámara. H: controles · F: pantalla completa · R: reiniciar. Ciudad: bandas de graves a agudos por columna.")
        hint.font = NSFont.systemFont(ofSize: 11)
        hint.widthAnchor.constraint(equalToConstant: 240).isActive = true
        let rows = NSStackView(views: [sceneRow,
            slider("Brillo", 1, 2, #selector(changeGlow(_:))),
            slider("Reactividad", 1, 3, #selector(changeSensitivity(_:))),
            slider("Velocidad", 1, 2, #selector(changeSpeed(_:))),
            slider("Color", 0, 6.283, #selector(changeHue(_:))), hint])
        rows.orientation = .vertical; rows.alignment = .leading; rows.spacing = 6
        controls = NSVisualEffectView()
        controls.material = .hudWindow; controls.blendingMode = .withinWindow; controls.state = .active
        rows.translatesAutoresizingMaskIntoConstraints = false
        controls.addSubview(rows)
        let toggle = NSButton(title: "Controles", target: self, action: #selector(toggleControls(_:)))
        toggle.keyEquivalent = "h"; toggle.toolTip = "Mostrar u ocultar controles (H)"
        let fullscreen = NSButton(title: "Pantalla completa", target: self, action: #selector(toggleFullscreen(_:)))
        fullscreen.keyEquivalent = "f"
        let reset = NSButton(title: "Reiniciar cámara", target: self, action: #selector(resetCamera(_:)))
        reset.keyEquivalent = "r"
        toolbar = NSStackView(views: [toggle, fullscreen, reset])
        toolbar.orientation = .horizontal; toolbar.spacing = 6
        view.translatesAutoresizingMaskIntoConstraints = false
        controls.translatesAutoresizingMaskIntoConstraints = false
        toolbar.translatesAutoresizingMaskIntoConstraints = false
        let content = window.contentView!
        content.addSubview(view); content.addSubview(controls); content.addSubview(toolbar)
        NSLayoutConstraint.activate([
            view.topAnchor.constraint(equalTo: content.topAnchor), view.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            view.trailingAnchor.constraint(equalTo: content.trailingAnchor), view.bottomAnchor.constraint(equalTo: content.bottomAnchor),
            toolbar.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12), toolbar.topAnchor.constraint(equalTo: content.topAnchor, constant: 10),
            controls.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12), controls.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 6),
            rows.topAnchor.constraint(equalTo: controls.topAnchor, constant: 12), rows.bottomAnchor.constraint(equalTo: controls.bottomAnchor, constant: -12),
            rows.leadingAnchor.constraint(equalTo: controls.leadingAnchor, constant: 12), rows.trailingAnchor.constraint(equalTo: controls.trailingAnchor, constant: -12)
        ])
        window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        DispatchQueue.global(qos: .utility).async { [weak self] in
            while let line = readLine() {
                guard let data = line.data(using: .utf8), let frame = try? JSONDecoder().decode(AudioFrame.self, from: data) else { continue }
                DispatchQueue.main.async {
                    guard let self = self else { return }
                    if let color = frame.accent, color.count == 3 { self.accent = SIMD4<Float>(color[0], color[1], color[2], 1) }
                    self.energy = frame.rms; self.peak = frame.peak
                    self.waveform = Array((frame.waveform + [Float](repeating: 0, count: 64)).prefix(64))
                    if let bins = frame.spectrum { self.spectrum = Array((bins + [Float](repeating: 0, count: 24)).prefix(24)) }
                }
            }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
    }
    @objc func changeScene(_ sender: NSPopUpButton) { scene = sender.indexOfSelectedItem }
    @objc func changeGlow(_ sender: NSSlider) { glow = sender.floatValue }
    @objc func changeSensitivity(_ sender: NSSlider) { sensitivity = sender.floatValue }
    @objc func changeSpeed(_ sender: NSSlider) { speed = sender.floatValue }
    @objc func changeHue(_ sender: NSSlider) { hue = sender.floatValue }
    @objc func toggleControls(_ sender: Any?) { controls.isHidden.toggle(); toolbar.isHidden = controls.isHidden }
    @objc func toggleFullscreen(_ sender: Any?) { window.toggleFullScreen(nil) }
    @objc func resetCamera(_ sender: Any?) { cameraTarget = .zero; camera = .zero; velocity = .zero }
    func draw(in view: MTKView) {
        let now = ProcessInfo.processInfo.systemUptime
        let dt = Float(min(max(now - lastFrame, 0), 0.05)); lastFrame = now
        guard window.isVisible, !window.isMiniaturized, let drawable = view.currentDrawable, let pass = view.currentRenderPassDescriptor, let buffer = queue.makeCommandBuffer(), let encoder = buffer.makeRenderCommandEncoder(descriptor: pass) else { return }
        animationTime += dt * speed
        velocity += ((cameraTarget - camera) * 32 - velocity * 10) * dt
        camera += velocity * dt
        let bass = spectrum[0..<7].max() ?? 0
        let mid = spectrum[7..<17].max() ?? 0
        let treble = spectrum[17..<24].max() ?? 0
        var u = Uniforms(time: animationTime, energy: energy, aspect: Float(view.drawableSize.width / max(1, view.drawableSize.height)), peak: peak, accent: accent, settings: SIMD4<Float>(Float(scene), glow, sensitivity, hue), camera: SIMD4<Float>(camera.x, camera.y, 0, 0), bands: SIMD4<Float>(bass * sensitivity, mid * sensitivity, treble * sensitivity, 0))
        encoder.setRenderPipelineState(scene == 0 ? landscapePipeline : (scene == 6 ? cityPipeline : scenePipeline))
        encoder.setDepthStencilState(depthState)
        encoder.setVertexBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        encoder.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        waveform.withUnsafeBytes { bytes in
            encoder.setVertexBytes(bytes.baseAddress!, length: bytes.count, index: 1)
            encoder.setFragmentBytes(bytes.baseAddress!, length: bytes.count, index: 1)
        }
        spectrum.withUnsafeBytes { bytes in
            encoder.setVertexBytes(bytes.baseAddress!, length: bytes.count, index: 2)
            encoder.setFragmentBytes(bytes.baseAddress!, length: bytes.count, index: 2)
        }
        let count = scene == 0 ? 96 * 96 * 6 : (scene == 6 ? 12 * 12 * 36 : 3)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: count)
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
