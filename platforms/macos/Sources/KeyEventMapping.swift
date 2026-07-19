import AppKit

func shouldForwardBackspaceDirectly(keyCode: UInt16, hasComposition: Bool) -> Bool {
    keyCode == 51 && !hasComposition
}

func candidateSelectionIndex(keyCode: UInt16, candidateCount: Int, pageStart: Int) -> Int? {
    let selectionKeyCodes: [UInt16] = [18, 19, 20, 21, 23, 22, 26, 28, 25]
    guard
        let visibleIndex = selectionKeyCodes.firstIndex(of: keyCode),
        pageStart >= 0,
        pageStart + visibleIndex < candidateCount
    else {
        return nil
    }
    return pageStart + visibleIndex
}

func printableInputScalar(from event: NSEvent) -> Unicode.Scalar? {
    guard event.type == .keyDown,
          let characters = event.characters,
          characters.unicodeScalars.count == 1,
          let scalar = characters.unicodeScalars.first,
          (33 ... 126).contains(scalar.value)
    else {
        return nil
    }

    return Unicode.Scalar(String(scalar).lowercased()) ?? scalar
}
