import AppKit
import Foundation

guard CommandLine.arguments.count == 2 else {
    fatalError("usage: GenerateIcon.swift OUTPUT")
}

let size = NSSize(width: 32, height: 32)
let image = NSImage(size: size)
image.lockFocus()

NSColor.black.setFill()
NSBezierPath(roundedRect: NSRect(origin: .zero, size: size), xRadius: 7, yRadius: 7).fill()

let attributes: [NSAttributedString.Key: Any] = [
    .font: NSFont.systemFont(ofSize: 20, weight: .semibold),
    .foregroundColor: NSColor.white,
]
let label = NSString(string: "あ")
let labelSize = label.size(withAttributes: attributes)
label.draw(
    at: NSPoint(x: (size.width - labelSize.width) / 2, y: (size.height - labelSize.height) / 2),
    withAttributes: attributes
)

image.unlockFocus()

guard let data = image.tiffRepresentation else {
    fatalError("failed to generate TIFF icon")
}
try data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]), options: .atomic)
