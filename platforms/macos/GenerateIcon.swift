import AppKit
import Foundation

guard CommandLine.arguments.count == 2 else {
    fatalError("usage: GenerateIcon.swift OUTPUT")
}

// Input source menu icons render at 16pt; provide 1x and 2x representations
// so the glyph stays sharp instead of being downscaled from one 32px bitmap.
let pointSize = NSSize(width: 16, height: 16)
let badgeColor = NSColor(srgbRed: 0.345, green: 0.337, blue: 0.839, alpha: 1)
let label = NSString(string: "あ")

func makeBitmap(pixels: Int) -> NSBitmapImageRep {
    guard let representation = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: pixels,
        pixelsHigh: pixels,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .calibratedRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ) else {
        fatalError("failed to allocate icon bitmap")
    }
    return representation
}

/// Measures where the glyph's inked pixels land relative to the point handed
/// to `draw(at:)`, because font line metrics place kana visibly off the
/// optical center of a badge this small.
func inkedBounds(fontSize: CGFloat) -> NSRect {
    let side = Int(fontSize * 3)
    let representation = makeBitmap(pixels: side)
    let origin = NSPoint(x: fontSize, y: fontSize)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: representation)
    label.draw(at: origin, withAttributes: [
        .font: NSFont.systemFont(ofSize: fontSize, weight: .semibold),
        .foregroundColor: NSColor.white,
    ])
    NSGraphicsContext.restoreGraphicsState()

    var minX = side
    var minY = side
    var maxX = -1
    var maxY = -1
    for y in 0..<side {
        for x in 0..<side where (representation.colorAt(x: x, y: y)?.alphaComponent ?? 0) > 0.1 {
            minX = min(minX, x)
            minY = min(minY, y)
            maxX = max(maxX, x)
            maxY = max(maxY, y)
        }
    }
    guard maxX >= minX else {
        fatalError("glyph rendered no pixels")
    }

    // Bitmap rows are top-down while the drawing context is bottom-up.
    return NSRect(
        x: CGFloat(minX) - origin.x,
        y: CGFloat(side - 1 - maxY) - origin.y,
        width: CGFloat(maxX - minX + 1),
        height: CGFloat(maxY - minY + 1)
    )
}

func render(scale: CGFloat) -> NSBitmapImageRep {
    let representation = makeBitmap(pixels: Int(pointSize.width * scale))
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: representation)
    let transform = NSAffineTransform()
    transform.scale(by: scale)
    transform.concat()

    let canvas = NSRect(origin: .zero, size: pointSize)
    badgeColor.setFill()
    NSBezierPath(roundedRect: canvas, xRadius: 3.5, yRadius: 3.5).fill()

    let fontSize: CGFloat = 11.5
    let glyph = inkedBounds(fontSize: fontSize * scale)
    label.draw(
        at: NSPoint(
            x: canvas.midX - (glyph.width / 2 + glyph.minX) / scale,
            y: canvas.midY - (glyph.height / 2 + glyph.minY) / scale
        ),
        withAttributes: [
            .font: NSFont.systemFont(ofSize: fontSize, weight: .semibold),
            .foregroundColor: NSColor.white,
        ]
    )
    NSGraphicsContext.restoreGraphicsState()
    // Declare the point size only after drawing so the context above maps
    // one unit to one pixel and the transform is the single scale factor.
    representation.size = pointSize
    return representation
}

let image = NSImage(size: pointSize)
image.addRepresentation(render(scale: 1))
image.addRepresentation(render(scale: 2))

guard let data = image.tiffRepresentation else {
    fatalError("failed to generate TIFF icon")
}
try data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]), options: .atomic)
