// Draws the app icon: the cue strip seen from afar, current card lit.
// Usage: swift scripts/icon.swift assets/AppIcon.icns
import AppKit

let out = CommandLine.arguments[1]
let iconset = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("AppIcon.iconset")
try? FileManager.default.removeItem(at: iconset)
try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)

func draw(_ px: Int) -> Data {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let s = CGFloat(px) / 1024
    let tile = NSRect(x: 100 * s, y: 100 * s, width: 824 * s, height: 824 * s)
    let tilePath = NSBezierPath(roundedRect: tile, xRadius: 185 * s, yRadius: 185 * s)
    NSGradient(starting: NSColor(srgbRed: 0.10, green: 0.13, blue: 0.20, alpha: 1),
               ending: NSColor(srgbRed: 0.03, green: 0.04, blue: 0.07, alpha: 1))!
        .draw(in: tilePath, angle: -90)

    let accent = NSColor(srgbRed: 0.30, green: 0.76, blue: 1.0, alpha: 1)
    func card(_ cx: CGFloat, _ w: CGFloat, _ h: CGFloat, _ alpha: CGFloat, lit: Bool) {
        let r = NSRect(x: (cx - w / 2) * s, y: (512 - h / 2) * s, width: w * s, height: h * s)
        let path = NSBezierPath(roundedRect: r, xRadius: 26 * s, yRadius: 26 * s)
        NSColor(white: 1, alpha: lit ? 0.16 : 0.08 * alpha).setFill()
        path.fill()
        if lit {
            accent.setStroke()
            path.lineWidth = 14 * s
            path.stroke()
        }
        let ink = NSColor(white: 1, alpha: lit ? 0.95 : 0.45 * alpha)
        ink.setFill()
        let pad = w * 0.14
        NSBezierPath(roundedRect: NSRect(x: r.minX + pad * s, y: r.maxY - (pad + h * 0.16) * s,
                                         width: (w * 0.62) * s, height: h * 0.11 * s),
                     xRadius: 8 * s, yRadius: 8 * s).fill()
        for i in 0..<2 {
            let y = r.maxY - (pad + h * (0.36 + 0.18 * CGFloat(i))) * s
            NSBezierPath(roundedRect: NSRect(x: r.minX + pad * s, y: y, width: (w * (0.7 - 0.2 * CGFloat(i))) * s,
                                             height: h * 0.07 * s), xRadius: 6 * s, yRadius: 6 * s).fill()
        }
    }
    card(232, 150, 200, 0.6, lit: false)
    card(792, 190, 250, 1, lit: false)
    card(512, 300, 380, 1, lit: true)
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

for (name, px) in [("16x16", 16), ("16x16@2x", 32), ("32x32", 32), ("32x32@2x", 64), ("128x128", 128),
                   ("128x128@2x", 256), ("256x256", 256), ("256x256@2x", 512), ("512x512", 512),
                   ("512x512@2x", 1024)] {
    try draw(px).write(to: iconset.appendingPathComponent("icon_\(name).png"))
}
let task = Process()
task.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
task.arguments = ["-c", "icns", iconset.path, "-o", out]
try task.run()
task.waitUntilExit()
exit(task.terminationStatus)
