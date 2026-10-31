// A separate process so that the camera can never outlive its use: killing it releases the device
// whatever state capture is in, and a failure here cannot reach the app or the audio.
// Prints one JSON object per line. Reads "preview 0" or "preview 1"; end of input means the app is
// gone, and it exits with it. Exit codes: 3 permission denied, 4 no camera, 1 anything else.
import AVFoundation
import CoreImage
import Foundation
import Vision

func emit(_ object: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: object) else { return }
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write(Data([0x0a]))
}

final class Tracker: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate {
    private let session = AVCaptureSession()
    private let queue = DispatchQueue(label: "curado.tracker.frames")
    private let tracksHands: Bool
    private let hands = VNDetectHumanHandPoseRequest()
    private let faces = VNDetectFaceRectanglesRequest()
    private let images = CIContext(options: [.useSoftwareRenderer: false])
    private var lastFrame = CMTime.invalid
    private var lastPreview = CMTime.invalid
    private let lock = NSLock()
    private var previewing = false
    /// Vision costs far more than capture, and hands do not need more than this.
    private let frameInterval = CMTime(value: 1, timescale: 15)
    private let previewInterval = CMTime(value: 1, timescale: 8)

    init(tracksHands: Bool) {
        self.tracksHands = tracksHands
        hands.maximumHandCount = 2
    }
    func setPreview(_ on: Bool) {
        lock.lock()
        previewing = on
        lock.unlock()
    }
    func start() {
        guard let camera = AVCaptureDevice.default(for: .video),
            let input = try? AVCaptureDeviceInput(device: camera)
        else {
            emit(["status": "nocamera"])
            exit(4)
        }
        session.beginConfiguration()
        if session.canSetSessionPreset(.vga640x480) { session.sessionPreset = .vga640x480 }
        guard session.canAddInput(input) else {
            emit(["status": "busy"])
            exit(1)
        }
        session.addInput(input)
        let output = AVCaptureVideoDataOutput()
        output.alwaysDiscardsLateVideoFrames = true
        output.setSampleBufferDelegate(self, queue: queue)
        guard session.canAddOutput(output) else {
            emit(["status": "busy"])
            exit(1)
        }
        session.addOutput(output)
        session.commitConfiguration()
        let center = NotificationCenter.default
        for name in [Notification.Name.AVCaptureSessionRuntimeError, .AVCaptureDeviceWasDisconnected] {
            // Started again from outside: a new process starts from a clean device.
            center.addObserver(forName: name, object: nil, queue: nil) { _ in
                emit(["status": "lost"])
                exit(1)
            }
        }
        session.startRunning()
        emit(["status": "running"])
    }
    func captureOutput(
        _ output: AVCaptureOutput, didOutput buffer: CMSampleBuffer, from connection: AVCaptureConnection
    ) {
        let now = CMSampleBufferGetPresentationTimeStamp(buffer)
        if lastFrame.isValid, CMTimeCompare(CMTimeSubtract(now, lastFrame), frameInterval) < 0 { return }
        lastFrame = now
        guard let pixels = CMSampleBufferGetImageBuffer(buffer) else { return }
        let handler = VNImageRequestHandler(cvPixelBuffer: pixels, orientation: .up, options: [:])
        var frame: [String: Any] = [:]
        if tracksHands {
            try? handler.perform([hands])
            frame["hands"] = (hands.results ?? []).compactMap(handPoint)
        } else {
            try? handler.perform([faces])
            if let face = (faces.results ?? []).max(by: {
                $0.boundingBox.width < $1.boundingBox.width
            }) {
                let box = face.boundingBox
                frame["face"] = ["x": Double(box.midX), "y": Double(box.midY), "size": Double(box.width)]
            }
        }
        lock.lock()
        let wantsPreview = previewing
        lock.unlock()
        if wantsPreview,
            !lastPreview.isValid || CMTimeCompare(CMTimeSubtract(now, lastPreview), previewInterval) >= 0,
            let jpeg = preview(pixels)
        {
            lastPreview = now
            frame["jpeg"] = jpeg
        }
        emit(frame)
    }
    private func handPoint(_ hand: VNHumanHandPoseObservation) -> [String: Double]? {
        guard let points = try? hand.recognizedPoints(.all),
            let wrist = points[.wrist], let knuckle = points[.middleMCP],
            let thumb = points[.thumbTip], let index = points[.indexTip],
            wrist.confidence > 0.3, knuckle.confidence > 0.3, thumb.confidence > 0.3, index.confidence > 0.3
        else { return nil }
        let distance = { (a: VNRecognizedPoint, b: VNRecognizedPoint) -> Double in
            Double(hypot(a.location.x - b.location.x, a.location.y - b.location.y))
        }
        let size = distance(wrist, knuckle)
        guard size > 0.001 else { return nil }
        return [
            "x": Double(knuckle.location.x), "y": Double(knuckle.location.y),
            "pinch": distance(thumb, index) / size, "size": size,
        ]
    }
    private func preview(_ pixels: CVPixelBuffer) -> String? {
        let image = CIImage(cvPixelBuffer: pixels)
        let scale = 160 / image.extent.width
        let small = image.transformed(by: CGAffineTransform(scaleX: scale, y: scale))
        guard let space = CGColorSpace(name: CGColorSpace.sRGB),
            let data = images.jpegRepresentation(
                of: small, colorSpace: space,
                options: [kCGImageDestinationLossyCompressionQuality as CIImageRepresentationOption: 0.5])
        else { return nil }
        return data.base64EncodedString()
    }
}

let tracker = Tracker(tracksHands: CommandLine.arguments.dropFirst().first != "face")
switch AVCaptureDevice.authorizationStatus(for: .video) {
case .authorized: tracker.start()
case .notDetermined:
    emit(["status": "asking"])
    AVCaptureDevice.requestAccess(for: .video) { granted in
        if granted {
            tracker.start()
        } else {
            emit(["status": "denied"])
            exit(3)
        }
    }
default:
    emit(["status": "denied"])
    exit(3)
}
DispatchQueue.global(qos: .utility).async {
    while let line = readLine() {
        if line == "preview 1" { tracker.setPreview(true) }
        if line == "preview 0" { tracker.setPreview(false) }
    }
    exit(0)
}
dispatchMain()
