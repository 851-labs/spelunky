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

struct Target {
    let pid: pid_t
    /// The game's back buffer size, when coordinates are in its pixels; `nil` for the content view's points.
    let screen: (Double, Double)?
    // A private source: the events carry only the flags this tool sets, not the real keyboard's.
    let source = CGEventSource(stateID: .privateState)!
    var flags: CGEventFlags = []
    var buttonsDown: Set<String> = []
    /// The last mouse position, for the wheel: script coordinates.
    var last = (0.0, 0.0)

    /// The app's main window: its number, its frame and its content area, in global points (top-left
    /// origin).
    func window() -> (number: Int, frame: CGRect, content: CGRect) {
        guard let list = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] else {
            fail("can't list windows")
        }
        var best: (Int, CGRect)?
        for info in list {
            guard (info[kCGWindowOwnerPID as String] as? Int).map(pid_t.init) == pid,
                (info[kCGWindowLayer as String] as? Int) == 0,
                let number = info[kCGWindowNumber as String] as? Int,
                let dict = info[kCGWindowBounds as String] as? NSDictionary,
                let bounds = CGRect(dictionaryRepresentation: dict as CFDictionary)
            else { continue }
            if best.map({ bounds.width * bounds.height > $0.1.width * $0.1.height }) ?? true {
                best = (number, bounds)
            }
        }
        guard let (number, frame) = best else { fail("process \(pid) has no window") }
        // The frame includes the title bar; the content is what's under it.
        let probe = NSRect(x: 0, y: 0, width: 100, height: 100)
        let titled = NSWindow.frameRect(
            forContentRect: probe, styleMask: [.titled, .closable, .miniaturizable, .resizable])
        let title = titled.height - probe.height
        let content = CGRect(x: frame.minX, y: frame.minY + title, width: frame.width,
                             height: frame.height - title)
        return (number, frame, content)
    }

    /// Where script coordinates are: back-buffer pixels with --screen (the app shows the buffer scaled to
    /// fit and centred), else content points. Aims at the pixel's centre. Gives the window, the point in
    /// global coordinates, and the point in the window's frame.
    func point(_ x: Double, _ y: Double) -> Aim {
        let (number, frame, content) = window()
        var at = CGPoint(x: content.minX + x, y: content.minY + y)
        if let (w, h) = screen {
            let scale = min(content.width / w, content.height / h)
            let left = content.minX + (content.width - w * scale) / 2
            let top = content.minY + (content.height - h * scale) / 2
            at = CGPoint(x: left + (x + 0.5) * scale, y: top + (y + 0.5) * scale)
        }
        return Aim(window: number, global: at, local: CGPoint(x: at.x - frame.minX, y: at.y - frame.minY))
    }

    func post(_ event: CGEvent) {
        event.flags = flags
        event.postToPid(pid)
    }

    mutating func key(_ name: String, down: Bool) {
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

    mutating func mouse(_ button: String, _ x: Double, _ y: Double, down: Bool?) {
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

    mutating func wheel(_ lines: Int32) {
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

/// A mouse position: the window it's in, and the point in global coordinates and in the window's frame.
struct Aim {
    let window: Int
    let global: CGPoint
    let local: CGPoint
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
        guard parts.count == 2 else { fail("--screen needs WxH") }
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

var target = Target(pid: pid, screen: screen)
let text: String
if let scriptPath {
    guard let t = try? String(contentsOfFile: scriptPath, encoding: .utf8) else { fail("can't read \(scriptPath)") }
    text = t
} else {
    text = String(data: FileHandle.standardInput.readDataToEndOfFile(), encoding: .utf8) ?? ""
}

let defaultHold = 100.0
for (n, raw) in text.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
    let line = raw.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false).first ?? ""
    let words = line.split(separator: " ").map(String.init)
    guard let command = words.first else { continue }
    let rest = Array(words.dropFirst())
    func number(_ i: Int, _ fallback: Double? = nil) -> Double {
        if i < rest.count, let v = Double(rest[i]) { return v }
        if let fallback { return fallback }
        fail("line \(n + 1): \(command) needs a number at argument \(i + 1)")
    }
    switch command {
    case "wait": wait(number(0))
    case "down": target.key(rest.first ?? "", down: true)
    case "up": target.key(rest.first ?? "", down: false)
    case "hold":
        // Down, held across frames, then up.
        target.key(rest.first ?? "", down: true)
        wait(number(1, defaultHold))
        target.key(rest.first ?? "", down: false)
    case "type":
        // Each character held and released in turn, with a gap as long as the hold.
        let ms = number(1, defaultHold)
        for c in rest.first ?? "" {
            let name = c == " " ? "space" : String(c)
            target.key(name, down: true)
            wait(ms)
            target.key(name, down: false)
            wait(ms)
        }
    case "move": target.mouse("", number(0), number(1), down: nil)
    case "press": target.mouse(rest.first ?? "", number(1), number(2), down: true)
    case "release": target.mouse(rest.first ?? "", number(1), number(2), down: false)
    case "click":
        let button = rest.first ?? ""
        target.mouse(button, number(1), number(2), down: true)
        wait(number(3, defaultHold))
        target.mouse(button, number(1), number(2), down: false)
    case "wheel":
        switch rest.first {
        case "up": target.wheel(1)
        case "down": target.wheel(-1)
        default: fail("line \(n + 1): wheel up or wheel down")
        }
    case "shot":
        // The window's own picture, even when other windows cover it.
        guard let path = rest.first else { fail("line \(n + 1): shot needs a path") }
        let number = target.window().number
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        p.arguments = ["-x", "-o", "-l", String(number), path]
        try? p.run()
        p.waitUntilExit()
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
