//! Just enough of a D-Bus client for [`crate::awake`] on Linux: `org.freedesktop.ScreenSaver.Inhibit` and
//! `UnInhibit` on the session bus (851-3873). The inhibit lasts while the connection that asked for it is
//! open, so the connection is kept for as long as it's held. The D-Bus crates either link the system's
//! `libdbus` or bring an async runtime; this is two method calls, little-endian, over the bus's Unix socket
//! with the `EXTERNAL` authentication (the D-Bus specification, "Message Protocol" and "Authentication
//! Protocol").
// Only Linux and the other freedesktop Unixes call it; macOS compiles it for its tests.
#![cfg_attr(target_os = "macos", allow(dead_code))]

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

/// How long a call may take: the inhibit is asked for as the window gets focus, so a bus that doesn't
/// answer mustn't hold the game up for long.
const TIMEOUT: Duration = Duration::from_secs(1);

/// A method call's message type, and a reply's or error's.
const METHOD_CALL: u8 = 1;
const METHOD_RETURN: u8 = 2;
const ERROR: u8 = 3;
/// Don't start the service to answer: a desktop without a screen saver service has nothing to inhibit.
const NO_AUTO_START: u8 = 2;
/// The header fields a call names.
const PATH: u8 = 1;
const INTERFACE: u8 = 2;
const MEMBER: u8 = 3;
const REPLY_SERIAL: u8 = 5;
const DESTINATION: u8 = 6;
const SIGNATURE: u8 = 8;

/// A connection to the session bus.
pub struct Bus {
    stream: UnixStream,
    serial: u32,
}

/// A method's arguments: strings and 32-bit unsigned integers are all these calls take.
pub enum Arg<'a> {
    Str(&'a str),
    U32(u32),
}

impl Bus {
    /// Connects to the session bus (`DBUS_SESSION_BUS_ADDRESS`, or `$XDG_RUNTIME_DIR/bus`) and says hello.
    pub fn session() -> io::Result<Bus> {
        let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        let stream = match address.as_deref().and_then(unix_address) {
            Some(Socket::Path(path)) => UnixStream::connect(path)?,
            #[cfg(target_os = "linux")]
            Some(Socket::Abstract(name)) => {
                use std::os::linux::net::SocketAddrExt;
                let addr = std::os::unix::net::SocketAddr::from_abstract_name(name)?;
                UnixStream::connect_addr(&addr)?
            }
            #[cfg(not(target_os = "linux"))]
            Some(Socket::Abstract(_)) => return Err(io::ErrorKind::Unsupported.into()),
            None => {
                let dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or(io::ErrorKind::NotFound)?;
                UnixStream::connect(std::path::Path::new(&dir).join("bus"))?
            }
        };
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        let mut bus = Bus { stream, serial: 0 };
        bus.authenticate()?;
        bus.call(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "Hello",
            &[],
        )?;
        Ok(bus)
    }

    /// `EXTERNAL`: the bus takes the user from the socket's peer credentials. Sent with no user named, it asks
    /// for one (`DATA`), and an empty answer lets it take the peer's, as sd-bus does: no uid to look up, which
    /// std can't without `/proc`.
    fn authenticate(&mut self) -> io::Result<()> {
        self.stream.write_all(b"\0AUTH EXTERNAL\r\n")?;
        let mut line = self.auth_line()?;
        if line.trim_end() == "DATA" {
            self.stream.write_all(b"DATA\r\n")?;
            line = self.auth_line()?;
        }
        if !line.starts_with("OK ") {
            return Err(io::Error::other(format!(
                "D-Bus refused: {}",
                line.trim_end()
            )));
        }
        self.stream.write_all(b"BEGIN\r\n")
    }

    /// One line of the authentication's: the bus sends one at a time, each answering the client's last.
    fn auth_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        BufReader::new(&self.stream)
            .take(512)
            .read_line(&mut line)?;
        Ok(line)
    }

    /// Calls a method and waits for its reply, whose body it returns. An error reply is an error.
    pub fn call(
        &mut self,
        destination: &str,
        path: &str,
        interface: &str,
        member: &str,
        args: &[Arg],
    ) -> io::Result<Message> {
        self.serial += 1;
        let serial = self.serial;
        self.stream.write_all(&method_call(
            serial,
            destination,
            path,
            interface,
            member,
            args,
        ))?;
        // The whole call within the timeout, however many other messages come first.
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            self.stream.set_read_timeout(Some(left))?;
            let message = read_message(&mut self.stream)?;
            // Skip a signal (the bus's NameAcquired after Hello), or a reply to something else.
            if message.reply_to == Some(serial) {
                match message.kind {
                    METHOD_RETURN => return Ok(message),
                    ERROR => return Err(io::Error::other(format!("D-Bus {member}: error reply"))),
                    _ => {}
                }
            }
        }
    }

    /// Sends a call without waiting for its reply (on the way out, where there's nothing to do with one).
    pub fn send(
        &mut self,
        destination: &str,
        path: &str,
        interface: &str,
        member: &str,
        args: &[Arg],
    ) -> io::Result<()> {
        self.serial += 1;
        self.stream.write_all(&method_call(
            self.serial,
            destination,
            path,
            interface,
            member,
            args,
        ))
    }
}

enum Socket<'a> {
    Path(&'a str),
    Abstract(&'a [u8]),
}

/// The first `unix:` address of a bus address list (`unix:path=...` or `unix:abstract=...`), unescaped only
/// where it needs nothing unescaped (an address with a `%` escape is skipped).
fn unix_address(list: &str) -> Option<Socket<'_>> {
    list.split(';').find_map(|address| {
        let keys = address.strip_prefix("unix:")?;
        keys.split(',').find_map(|kv| {
            let (key, value) = kv.split_once('=')?;
            if value.contains('%') {
                return None;
            }
            match key {
                "path" => Some(Socket::Path(value)),
                "abstract" => Some(Socket::Abstract(value.as_bytes())),
                _ => None,
            }
        })
    })
}

/// Pads `out` with zeros to a multiple of `n`.
fn align(out: &mut Vec<u8>, n: usize) {
    while !out.len().is_multiple_of(n) {
        out.push(0);
    }
}

/// A string (`s` or `o`): its length, its bytes and a nul, aligned to 4.
fn put_str(out: &mut Vec<u8>, s: &str) {
    align(out, 4);
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

/// A signature (`g`): its length in a byte, its bytes and a nul.
fn put_signature(out: &mut Vec<u8>, s: &str) {
    out.push(s.len() as u8);
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

/// A method call's message, little-endian.
fn method_call(
    serial: u32,
    destination: &str,
    path: &str,
    interface: &str,
    member: &str,
    args: &[Arg],
) -> Vec<u8> {
    let mut body = Vec::new();
    let mut signature = String::new();
    for arg in args {
        match arg {
            Arg::Str(s) => {
                signature.push('s');
                put_str(&mut body, s);
            }
            Arg::U32(u) => {
                signature.push('u');
                align(&mut body, 4);
                body.extend_from_slice(&u.to_le_bytes());
            }
        }
    }
    let mut out = vec![b'l', METHOD_CALL, NO_AUTO_START, 1];
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&serial.to_le_bytes());
    // The header fields' array: its length is filled in once it's written.
    out.extend_from_slice(&[0; 4]);
    let fields = out.len();
    let field = |out: &mut Vec<u8>, code: u8, kind: &str, value: &str| {
        align(out, 8);
        out.push(code);
        put_signature(out, kind);
        if kind == "g" {
            put_signature(out, value);
        } else {
            put_str(out, value);
        }
    };
    field(&mut out, PATH, "o", path);
    field(&mut out, INTERFACE, "s", interface);
    field(&mut out, MEMBER, "s", member);
    field(&mut out, DESTINATION, "s", destination);
    if !signature.is_empty() {
        field(&mut out, SIGNATURE, "g", &signature);
    }
    let length = (out.len() - fields) as u32;
    out[12..16].copy_from_slice(&length.to_le_bytes());
    align(&mut out, 8);
    out.extend_from_slice(&body);
    out
}

/// A message read: its type, the serial it replies to (if it's a reply or an error), its body, and whether
/// its numbers are big-endian (the sender's order, which the bus passes on).
pub struct Message {
    kind: u8,
    reply_to: Option<u32>,
    body: Vec<u8>,
    big: bool,
}

impl Message {
    /// The `u32` the body starts with.
    pub fn u32(&self) -> Option<u32> {
        let bytes = self.body.get(..4)?.try_into().ok()?;
        Some(if self.big {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        })
    }
}

fn truncated() -> io::Error {
    io::Error::other("D-Bus: a truncated message")
}

/// Reads one message.
fn read_message(stream: &mut impl Read) -> io::Result<Message> {
    let mut fixed = [0u8; 16];
    stream.read_exact(&mut fixed)?;
    let big = match fixed[0] {
        b'l' => false,
        b'B' => true,
        _ => return Err(io::Error::other("D-Bus: not a message")),
    };
    let u32_at = |b: &[u8], at: usize| -> io::Result<u32> {
        let bytes: [u8; 4] = b
            .get(at..at + 4)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(truncated)?;
        Ok(if big {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        })
    };
    let body_len = u32_at(&fixed, 4)? as usize;
    let fields_len = u32_at(&fixed, 12)? as usize;
    if body_len > 1 << 20 || fields_len > 1 << 20 {
        return Err(io::Error::other("D-Bus: a message too large"));
    }
    // The fields, then padding to 8, counted from the message's start.
    let padded = (16 + fields_len).div_ceil(8) * 8;
    let mut rest = vec![0u8; padded - 16 + body_len];
    stream.read_exact(&mut rest)?;
    let mut message = fixed.to_vec();
    message.extend_from_slice(&rest);
    let fields = &message[..16 + fields_len];
    let byte = |at: usize| fields.get(at).copied().ok_or_else(truncated);
    let mut at = 16;
    let mut reply_to = None;
    while at < fields.len() {
        at = at.div_ceil(8) * 8;
        let code = byte(at)?;
        let sig_len = byte(at + 1)?;
        let kind = byte(at + 2)?;
        at += 2 + sig_len as usize + 1;
        match (sig_len, kind) {
            (1, b'u') => {
                at = at.div_ceil(4) * 4;
                if code == REPLY_SERIAL {
                    reply_to = Some(u32_at(fields, at)?);
                }
                at += 4;
            }
            (1, b's' | b'o') => {
                at = at.div_ceil(4) * 4;
                at += 4 + u32_at(fields, at)? as usize + 1;
            }
            (1, b'g') => at += 1 + byte(at)? as usize + 1,
            _ => return Err(io::Error::other("D-Bus: a header field of an unknown type")),
        }
    }
    Ok(Message {
        kind: fixed[1],
        reply_to,
        body: message[padded..].to_vec(),
        big,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_method_call_is_marshalled_as_the_specification_lays_it_out() {
        let m = method_call(7, "d", "/p", "i", "m", &[Arg::Str("ab"), Arg::U32(5)]);
        // Endianness, type, flags, version; body length (4 + 2 + 1, then 1 of padding, then 4); serial.
        assert_eq!(&m[..12], &[b'l', 1, 2, 1, 12, 0, 0, 0, 7, 0, 0, 0]);
        // The fields: PATH "/p" (o), aligned to 8 from 16.
        assert_eq!(&m[16..28], &[1, 1, b'o', 0, 2, 0, 0, 0, b'/', b'p', 0, 0]);
        // The last field, SIGNATURE "su", then padding to 8, then the body.
        let body = &m[m.len() - 12..];
        assert_eq!(body, &[2, 0, 0, 0, b'a', b'b', 0, 0, 5, 0, 0, 0]);
        assert_eq!((m.len() - 12) % 8, 0, "the body starts on 8");
        let fields_len = u32::from_le_bytes(m[12..16].try_into().unwrap()) as usize;
        assert_eq!((16 + fields_len).div_ceil(8) * 8, m.len() - 12);
        assert!(
            m[16..16 + fields_len]
                .windows(5)
                .any(|w| w == [8, 1, b'g', 0, 2])
        );
    }

    #[test]
    fn a_reply_is_read_with_the_serial_it_answers() {
        // A method return to serial 3 with a u32 body, 42: fields REPLY_SERIAL (u) and SIGNATURE (g).
        let mut m = vec![b'l', METHOD_RETURN, 1, 1, 4, 0, 0, 0, 9, 0, 0, 0];
        let mut fields = vec![REPLY_SERIAL, 1, b'u', 0, 3, 0, 0, 0];
        fields.extend_from_slice(&[SIGNATURE, 1, b'g', 0, 1, b'u', 0]);
        m.extend_from_slice(&(fields.len() as u32).to_le_bytes());
        m.extend_from_slice(&fields);
        align(&mut m, 8);
        m.extend_from_slice(&42u32.to_le_bytes());
        let r = read_message(&mut m.as_slice()).unwrap();
        assert_eq!(
            (r.kind, r.reply_to, r.u32()),
            (METHOD_RETURN, Some(3), Some(42))
        );
        // The same, big-endian.
        let mut b = vec![b'B', METHOD_RETURN, 1, 1, 0, 0, 0, 4, 0, 0, 0, 9];
        let mut fields = vec![REPLY_SERIAL, 1, b'u', 0, 0, 0, 0, 3];
        fields.extend_from_slice(&[SIGNATURE, 1, b'g', 0, 1, b'u', 0]);
        b.extend_from_slice(&(fields.len() as u32).to_be_bytes());
        b.extend_from_slice(&fields);
        align(&mut b, 8);
        b.extend_from_slice(&42u32.to_be_bytes());
        let r = read_message(&mut b.as_slice()).unwrap();
        assert_eq!((r.reply_to, r.u32()), (Some(3), Some(42)));
        // A message as `method_call` writes it reads back too, with no reply serial.
        let call = method_call(1, "d", "/p", "i", "m", &[Arg::Str("x")]);
        let r = read_message(&mut call.as_slice()).unwrap();
        assert_eq!((r.kind, r.reply_to, r.body.len()), (METHOD_CALL, None, 6));
        // A fields array that runs past its own end is an error, not a panic.
        let mut short = m.clone();
        short[12] = 9;
        assert!(read_message(&mut short.as_slice()).is_err());
    }

    #[test]
    fn bus_addresses() {
        assert!(matches!(
            unix_address("unix:path=/run/user/1000/bus"),
            Some(Socket::Path("/run/user/1000/bus"))
        ));
        assert!(matches!(
            unix_address("tcp:host=x;unix:abstract=/tmp/dbus-1,guid=ab"),
            Some(Socket::Abstract(b"/tmp/dbus-1"))
        ));
        assert!(unix_address("unix:path=/a%20b").is_none());
        assert!(unix_address("tcp:host=localhost,port=1").is_none());
    }
}
