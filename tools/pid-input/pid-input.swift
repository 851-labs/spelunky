// Drives one running app with real, held input: key-down, a hold across frames, key-up, and mouse and wheel
// events, all posted to that process alone (CGEventPostToPid), never to the system's event stream. So it
// can drive the game while someone else uses the machine, and a key held across a tick types where an
// instant tap wouldn't (docs/quirks.md section 5). See README.md.
//
//   swiftc -O -o pid-input tools/pid-input/pid-input.swift
//   pid-input --pid PID [--screen WxH] [SCRIPT]       (the script on stdin without SCRIPT)

import AppKit
import CoreGraphics
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write("pid-input: \(message)\n".data(using: .utf8)!)
    exit(1)
}

// macOS virtual key codes (HIToolbox's kVK_*), by the names scripts use.
let keyCodes: [String: CGKeyCode] = {
    var codes: [String: CGKeyCode] = [
        "a": 0x00, "s": 0x01, "d": 0x02, "f": 0x03, "h": 0x04, "g": 0x05, "z": 0x06, "x": 0x07,
        "c": 0x08, "v": 0x09, "b": 0x0B, "q": 0x0C, "w": 0x0D, "e": 0x0E, "r": 0x0F, "y": 0x10,
        "t": 0x11, "1": 0x12, "2": 0x13, "3": 0x14, "4": 0x15, "6": 0x16, "5": 0x17, "=": 0x18,
        "9": 0x19, "7": 0x1A, "-": 0x1B, "8": 0x1C, "0": 0x1D, "]": 0x1E, "o": 0x1F, "u": 0x20,
        "[": 0x21, "i": 0x22, "p": 0x23, "l": 0x25, "j": 0x26, "'": 0x27, "k": 0x28, ";": 0x29,
        "\\": 0x2A, ",": 0x2B, "/": 0x2C, "n": 0x2D, "m": 0x2E, ".": 0x2F, "`": 0x32,
        "enter": 0x24, "return": 0x24, "tab": 0x30, "space": 0x31, "backspace": 0x33, "escape": 0x35,
        "delete": 0x75, "home": 0x73, "end": 0x77, "pageup": 0x74, "pagedown": 0x79,
        "left": 0x7B, "right": 0x7C, "down": 0x7D, "up": 0x7E,
        "shift": 0x38, "control": 0x3B, "alt": 0x3A, "rshift": 0x3C, "rcontrol": 0x3E, "ralt": 0x3D,
    ]
    let fkeys: [CGKeyCode] = [0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F]
    for (i, code) in fkeys.enumerated() { codes["f\(i + 1)"] = code }
    return codes
}()

// Modifier keys arrive as flag changes, not key downs, and each holds its flag while down.
let modifierFlags: [CGKeyCode: CGEventFlags] = [
    0x38: .maskShift, 0x3C: .maskShift, 0x3B: .maskControl, 0x3E: .maskControl,
    0x3A: .maskAlternate, 0x3D: .maskAlternate,
]

/// A mouse position: the window it's in, and the point in global coordinates and in the window's frame.
struct Aim {
    let window: CGWindowID
    let global: CGPoint
    let local: CGPoint
}

/// A window's bounds in global points (top-left origin), from a window-list entry.
func bounds(_ info: [String: Any]) -> CGRect? {
    guard let dict = info[kCGWindowBounds as String] as? NSDictionary else { return nil }
    return CGRect(dictionaryRepresentation: dict as CFDictionary)
}

/// The display a frame (global points, top-left origin) is mostly on: its frame in the same coordinates and
/// its backing scale (physical pixels per point).
func display(of frame: CGRect) -> (CGRect, Double) {
    // NSScreen frames have a bottom-left origin at the main display's bottom.
    let mainHeight = NSScreen.screens.first?.frame.height ?? 0
    var best: (CGRect, Double, Double)?
    for screen in NSScreen.screens {
        let f = screen.frame
        let top = CGRect(x: f.minX, y: mainHeight - f.maxY, width: f.width, height: f.height)
        let overlap = top.intersection(frame)
        let area = overlap.isNull ? 0 : overlap.width * overlap.height
        if best.map({ area > $0.2 }) ?? true { best = (top, Double(screen.backingScaleFactor), area) }
    }
    return best.map { ($0.0, $0.1) } ?? (frame, 2)
}

final class Target {
    let pid: pid_t
    /// The game's back buffer size, when coordinates are in its pixels; `nil` for the content view's points.
    let screen: (Double, Double)?
    // A private source: the events carry only the flags this tool sets, not the real keyboard's.
    let source = CGEventSource(stateID: .privateState)!
    var flags: CGEventFlags = []
    var buttonsDown: Set<String> = []
    /// The last mouse position, for the wheel: script coordinates.
    var last = (0.0, 0.0)
    var windowNumber: CGWindowID?

    init(pid: pid_t, screen: (Double, Double)?) {
        self.pid = pid
        self.screen = screen
    }

    /// The app's window: the largest normal one on screen, found once (listing every window is slow, and a
    /// drag asks for it on every move).
    func findWindow() -> CGWindowID {
        if let windowNumber { return windowNumber }
        guard let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]]
        else { fail("can't list windows") }
        var best: (CGWindowID, Double)?
        for info in list {
            guard (info[kCGWindowOwnerPID as String] as? Int).map(pid_t.init) == pid,
                (info[kCGWindowLayer as String] as? Int) == 0,
                let number = info[kCGWindowNumber as String] as? Int,
                let frame = bounds(info)
            else { continue }
            let area = Double(frame.width * frame.height)
            if best.map({ area > $0.1 }) ?? true { best = (CGWindowID(number), area) }
        }
        guard let (number, _) = best else { fail("process \(pid) has no window on screen") }
        windowNumber = number
        return number
    }

    /// The window: its number, its frame and its content area in global points (top-left origin), and its
    /// display's backing scale.
    func window() -> (number: CGWindowID, frame: CGRect, content: CGRect, backing: Double) {
        let number = findWindow()
        var id = UnsafeRawPointer(bitPattern: UInt(number))
        guard let array = CFArrayCreate(nil, &id, 1, nil),
            let list = CGWindowListCreateDescriptionFromArray(array) as? [[String: Any]],
            let frame = list.first.flatMap(bounds)
        else { fail("window \(number) is gone") }
        let (displayFrame, backing) = display(of: frame)
        // A titled window's frame includes its title bar; a fullscreen one fills its display and has none.
        var title = 0.0
        if frame != displayFrame {
            let probe = NSRect(x: 0, y: 0, width: 100, height: 100)
            let titled = NSWindow.frameRect(
                forContentRect: probe, styleMask: [.titled, .closable, .miniaturizable, .resizable])
            title = titled.height - probe.height
        }
        let content = CGRect(x: frame.minX, y: frame.minY + title, width: frame.width,
                             height: frame.height - title)
        return (number, frame, content, backing)
    }

    /// Where script coordinates are: back-buffer pixels with --screen, else content points. The app shows
    /// the buffer at the largest whole scale that fits the window's physical pixels (below 1, the fraction
    /// that fits) with the margin floored (app::present::fit); this aims at the middle of the pixel there.
    func point(_ x: Double, _ y: Double) -> Aim {
        let (number, frame, content, backing) = window()
        var at = CGPoint(x: content.minX + x, y: content.minY + y)
        if let (w, h) = screen {
            let (pw, ph) = ((content.width * backing).rounded(), (content.height * backing).rounded())
            let whole = min((pw / w).rounded(.down), (ph / h).rounded(.down))
            let scale = whole >= 1 ? whole : min(pw / w, ph / h)
            let left = ((pw - w * scale) / 2).rounded(.down)
            let top = ((ph - h * scale) / 2).rounded(.down)
            at = CGPoint(x: content.minX + (left + (x + 0.5) * scale) / backing,
                         y: content.minY + (top + (y + 0.5) * scale) / backing)
        }
        return Aim(window: number, global: at, local: CGPoint(x: at.x - frame.minX, y: at.y - frame.minY))
    }

    func post(_ event: CGEvent) {
        event.flags = flags
        event.postToPid(pid)
    }

    func key(_ name: String, down: Bool) {
        guard let code = keyCodes[name.lowercased()] else { fail("unknown key \(name)") }
        guard let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down) else {
            fail("can't make a key event")
        }
        if let flag = modifierFlags[code] {
            if down { flags.insert(flag) } else { flags.remove(flag) }
            event.type = .flagsChanged
        }
        post(event)
    }

    func mouse(_ button: String, _ x: Double, _ y: Double, down: Bool?) {
        let at = point(x, y)
        let types: [String: (CGMouseButton, CGEventType, CGEventType, CGEventType)] = [
            "left": (.left, .leftMouseDown, .leftMouseUp, .leftMouseDragged),
            "right": (.right, .rightMouseDown, .rightMouseUp, .rightMouseDragged),
            "middle": (.center, .otherMouseDown, .otherMouseUp, .otherMouseDragged),
        ]
        let type: CGEventType
        var cgButton = CGMouseButton.left
        if let down {
            guard let t = types[button] else { fail("unknown mouse button \(button)") }
            cgButton = t.0
            type = down ? t.1 : t.2
            if down { buttonsDown.insert(button) } else { buttonsDown.remove(button) }
        } else if let held = ["left", "right", "middle"].first(where: buttonsDown.contains) {
            // A move with a button down is a drag, as AppKit reports it.
            cgButton = types[held]!.0
            type = types[held]!.3
        } else {
            type = .mouseMoved
        }
        guard let event = CGEvent(mouseEventSource: source, mouseType: type,
                                  mouseCursorPosition: at.global, mouseButton: cgButton)
        else { fail("can't make a mouse event") }
        if down != nil { event.setIntegerValueField(.mouseEventClickState, value: 1) }
        aim(event, at)
        last = (x, y)
        post(event)
    }

    func wheel(_ lines: Int32) {
        let at = point(last.0, last.1)
        guard let event = CGEvent(scrollWheelEvent2Source: source, units: .line, wheelCount: 1,
                                  wheel1: lines, wheel2: 0, wheel3: 0) else { fail("can't make a wheel event") }
        event.location = at.global
        aim(event, at)
        post(event)
    }

    /// Puts a mouse event in the window, which a posted event otherwise isn't (the real pointer is
    /// elsewhere, and stays there): AppKit drops a click or a wheel that has no window. It takes the window
    /// from the undocumented field 51 and the point in it from the private `CGEventSetWindowLocation` (in
    /// the window's frame, top-left origin), not from the event's global location.
    func aim(_ event: CGEvent, _ at: Aim) {
        event.setIntegerValueField(CGEventField(rawValue: 51)!, value: Int64(at.window))
        event.setIntegerValueField(.mouseEventWindowUnderMousePointer, value: Int64(at.window))
        event.setIntegerValueField(.mouseEventWindowUnderMousePointerThatCanHandleThisEvent,
                                   value: Int64(at.window))
        setWindowLocation(event, at.local)
    }
}

typealias SetWindowLocation = @convention(c) (CGEvent, CGPoint) -> Void
let setWindowLocation: SetWindowLocation = {
    // RTLD_DEFAULT: CoreGraphics is already loaded.
    guard let symbol = dlsym(UnsafeMutableRawPointer(bitPattern: -2), "CGEventSetWindowLocation") else {
        fail("this macOS has no CGEventSetWindowLocation, which mouse events need")
    }
    return unsafeBitCast(symbol, to: SetWindowLocation.self)
}()

/// Waits while the run loop turns, so AppKit state (the frontmost app) stays current.
func wait(_ ms: Double) {
    RunLoop.current.run(until: Date(timeIntervalSinceNow: ms / 1000))
}

func frontmost() -> String {
    let app = NSWorkspace.shared.frontmostApplication
    return "\(app?.localizedName ?? "?") (\(app?.processIdentifier ?? 0))"
}

var args = Array(CommandLine.arguments.dropFirst())
var pid: pid_t?
var screen: (Double, Double)?
var scriptPath: String?
while !args.isEmpty {
    let arg = args.removeFirst()
    switch arg {
    case "--pid":
        guard let v = args.first.flatMap({ pid_t($0) }) else { fail("--pid needs a process id") }
        args.removeFirst()
        pid = v
    case "--screen":
        let parts = (args.first ?? "").split(separator: "x").compactMap { Double($0) }
        guard parts.count == 2, parts[0] > 0, parts[1] > 0 else { fail("--screen needs WxH") }
        args.removeFirst()
        screen = (parts[0], parts[1])
    case "-h", "--help":
        print("usage: pid-input --pid PID [--screen WxH] [SCRIPT]")
        exit(0)
    default:
        scriptPath = arg
    }
}
guard let pid else { fail("--pid is required") }
guard NSRunningApplication(processIdentifier: pid) != nil else { fail("no app with pid \(pid)") }

let target = Target(pid: pid, screen: screen)
let text: String
if let scriptPath {
    guard let t = try? String(contentsOfFile: scriptPath, encoding: .utf8) else { fail("can't read \(scriptPath)") }
    text = t
} else {
    text = String(data: FileHandle.standardInput.readDataToEndOfFile(), encoding: .utf8) ?? ""
}

let defaultHold = 100.0
// Lines end in \n or \r\n (Swift's Character treats \r\n as one), and words are split on any whitespace.
for (n, raw) in text.split(omittingEmptySubsequences: false, whereSeparator: \.isNewline).enumerated() {
    let line = raw.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false).first ?? ""
    let words = line.split(whereSeparator: \.isWhitespace).map(String.init)
    guard let command = words.first else { continue }
    let rest = Array(words.dropFirst())
    func number(_ i: Int, _ fallback: Double? = nil) -> Double {
        if i < rest.count, let v = Double(rest[i]) { return v }
        if i >= rest.count, let fallback { return fallback }
        fail("line \(n + 1): \(command) needs a number at argument \(i + 1)")
    }
    func word(_ i: Int) -> String {
        guard i < rest.count else { fail("line \(n + 1): \(command) needs argument \(i + 1)") }
        return rest[i]
    }
    switch command {
    case "wait": wait(number(0))
    case "down": target.key(word(0), down: true)
    case "up": target.key(word(0), down: false)
    case "hold":
        // Down, held across frames, then up.
        let key = word(0)
        target.key(key, down: true)
        wait(number(1, defaultHold))
        target.key(key, down: false)
    case "type":
        // Each character of one word held and released in turn, with a gap as long as the hold.
        let ms = number(1, defaultHold)
        for c in word(0) {
            target.key(String(c), down: true)
            wait(ms)
            target.key(String(c), down: false)
            wait(ms)
        }
    case "move": target.mouse("", number(0), number(1), down: nil)
    case "press": target.mouse(word(0), number(1), number(2), down: true)
    case "release": target.mouse(word(0), number(1), number(2), down: false)
    case "click":
        let button = word(0)
        target.mouse(button, number(1), number(2), down: true)
        wait(number(3, defaultHold))
        target.mouse(button, number(1), number(2), down: false)
    case "wheel":
        switch word(0) {
        case "up": target.wheel(1)
        case "down": target.wheel(-1)
        default: fail("line \(n + 1): wheel up or wheel down")
        }
    case "shot":
        // The window's own picture, even when other windows cover it.
        let path = word(0)
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        p.arguments = ["-x", "-o", "-l", String(target.findWindow()), path]
        do { try p.run() } catch { fail("line \(n + 1): can't run screencapture: \(error)") }
        p.waitUntilExit()
        guard p.terminationStatus == 0, FileManager.default.fileExists(atPath: path) else {
            fail("line \(n + 1): screencapture didn't write \(path) (Screen Recording permission?)")
        }
    case "front": print("frontmost: \(frontmost())")
    case "pointer":
        // The real pointer, which this tool's mouse events leave where it is.
        let at = CGEvent(source: nil)?.location ?? .zero
        print("pointer: \(Int(at.x)),\(Int(at.y))")
    case "echo": print(rest.joined(separator: " "))
    default: fail("line \(n + 1): unknown command \(command)")
    }
    fflush(stdout)
}
