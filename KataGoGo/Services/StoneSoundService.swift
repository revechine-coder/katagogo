import AVFoundation

enum StoneSoundPreset: String, CaseIterable {
    case wood = "wood"
    case crisp = "crisp"
    case deep = "deep"
    case mute = "mute"

    var label: String {
        switch self {
        case .wood: "木盘"
        case .crisp: "清脆"
        case .deep: "厚重"
        case .mute: "静音"
        }
    }
}

final class StoneSoundService {
    static let shared = StoneSoundService()
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private let format = AVAudioFormat(standardFormatWithSampleRate: 44100, channels: 1)!
    private var buffers: [StoneSoundPreset: AVAudioPCMBuffer] = [:]

    private init() {
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        engine.mainMixerNode.volume = 0.7
        engine.prepare()
        rebuildAllBuffers()
        try? engine.start()
    }

    private func rebuildAllBuffers() {
        for preset in StoneSoundPreset.allCases {
            buffers[preset] = buildBuffer(for: preset)
        }
    }

    private func buildBuffer(for preset: StoneSoundPreset) -> AVAudioPCMBuffer? {
        let params: (duration: Float, pitch: Float, noiseWeight: Float, toneWeight: Float, decay: Float, volume: Float) = {
            switch preset {
            case .wood:  return (0.05, 1200, 0.50, 0.50, 70, 0.65)
            case .crisp: return (0.03, 2400, 0.30, 0.70, 100, 0.60)
            case .deep:  return (0.07, 600,  0.40, 0.60, 50, 0.70)
            case .mute:  return (0.01, 1,    0,    0,    1,  0)
            }
        }()

        let sampleCount = Int(params.duration * Float(format.sampleRate))
        guard let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(sampleCount)) else { return nil }
        buf.frameLength = AVAudioFrameCount(sampleCount)
        guard let channelData = buf.floatChannelData?[0] else { return nil }

        for i in 0..<sampleCount {
            let t = Float(i) / Float(sampleCount)
            let envelope = exp(-t * params.decay)
            let noise = Float.random(in: -1...1)
            let tone = sin(2 * .pi * params.pitch * t)
            channelData[i] = (noise * params.noiseWeight + tone * params.toneWeight) * envelope * params.volume
        }
        return buf
    }

    func playClick() {
        let preset = StoneSoundPreset(rawValue: AppState.shared.stoneSoundPreset) ?? .wood
        if preset == .mute { return }
        guard let buffer = buffers[preset] else { return }
        if !engine.isRunning {
            do { try engine.start() } catch { return }
        }
        player.scheduleBuffer(buffer, at: nil, options: .interrupts) {}
        if !player.isPlaying {
            player.play()
        }
    }
}
