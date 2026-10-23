//! Traces: the game state after every frame, as JSON lines in the oracle's schema.
//!
//! Each line is one [`Frame`], written after the frame's draw events and after any room change requested
//! during the frame has happened, so it is the state the next frame starts from. A trace may begin with an
//! optional [`TraceHeader`] line (an extension, recognized by its `format` key); the oracle doesn't write one.
//! Anything a [`Filter`] removed, or a writer doesn't dump, is simply absent. See `docs/formats.md`.

use std::collections::{BTreeMap, HashSet};
use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};

use crate::filter::Filter;
use crate::value::{Real, Value};
use crate::{Error, GAME_VERSION};

pub const TRACE_FORMAT: &str = "spelunky-trace";
pub const TRACE_VERSION: u32 = 1;

/// The number of alarms per instance (`alarm[0]` to `alarm[11]`).
pub const ALARMS: u32 = 12;

/// The optional first line of a trace (an extension; the oracle doesn't write one yet).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceHeader {
    /// Always [`TRACE_FORMAT`].
    pub format: String,
    /// [`TRACE_VERSION`]; readers reject versions they don't know.
    pub version: u32,
    /// Always [`GAME_VERSION`].
    pub game: String,
    /// Who wrote the trace, free-form (`"port"`, `"oracle"`, a commit hash...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The filter the writer applied, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
}

impl TraceHeader {
    pub fn new(source: Option<&str>) -> Self {
        TraceHeader {
            format: TRACE_FORMAT.to_string(),
            version: TRACE_VERSION,
            game: GAME_VERSION.to_string(),
            source: source.map(String::from),
            filter: None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != TRACE_FORMAT {
            return Err(format!(
                "format is {:?}, expected {TRACE_FORMAT:?}",
                self.format
            ));
        }
        if self.version != TRACE_VERSION {
            return Err(format!(
                "unsupported trace version {} (this reader knows {TRACE_VERSION})",
                self.version
            ));
        }
        if self.game != GAME_VERSION {
            return Err(format!(
                "game is {:?}, expected {GAME_VERSION:?}",
                self.game
            ));
        }
        Ok(())
    }
}

/// The state after one frame. Fields besides `frame` are optional so filters can drop them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    /// 0-based; frame `f` ran with the replay's events for frame `f`. Strictly increasing within a trace,
    /// though a trace may skip frames.
    pub frame: u64,
    /// The room's index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_name: Option<String>,
    /// The runner's RNG state (OpenGMK's `rand.seed()`), as the signed 32-bit value the runner holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    /// GM's `instance_count`: the number of active instances (before any object filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_count: Option<u64>,
    /// Active instances, sorted by id.
    #[serde(default)]
    pub instances: Vec<Instance>,
    /// Deactivated instances (`instance_deactivate_*`), sorted by id. An extension; absent means not dumped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deactivated: Option<Vec<Instance>>,
    /// `global.*` variables, keyed without the prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub globals: Option<BTreeMap<String, Value>>,
    /// Runner built-ins that are neither per instance nor `global.*`, keyed by GML name with any index
    /// written out: `view_xview[0]`, `keyboard_key`... An extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtins: Option<BTreeMap<String, Value>>,
}

/// One instance. Everything except `id` and `object` is optional so filters can drop it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub id: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_index: Option<i32>,
    /// The object's name.
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xprevious: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yprevious: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hspeed: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vspeed: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friction: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_index: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_speed: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_xscale: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_yscale: Option<Real>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<Real>,
    /// -1 for no sprite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_index: Option<i32>,
    /// The sprite's name; `""` for no sprite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solid: Option<bool>,
    /// Alarm index (0-11) to value. Like the oracle, only alarms that were ever set need appear; a missing
    /// alarm is -1 (off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarms: Option<BTreeMap<u32, i32>>,
    /// The instance's own variables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vars: Option<BTreeMap<String, Value>>,
}

impl Instance {
    /// The real-valued built-ins, in serialization order.
    pub const REAL_FIELDS: [&'static str; 15] = [
        "x",
        "y",
        "xprevious",
        "yprevious",
        "hspeed",
        "vspeed",
        "speed",
        "direction",
        "gravity",
        "friction",
        "image_index",
        "image_speed",
        "image_xscale",
        "image_yscale",
        "depth",
    ];

    /// An instance with no fields set.
    pub fn new(id: i32, object: &str) -> Self {
        Instance {
            id,
            object_index: None,
            object: object.to_string(),
            x: None,
            y: None,
            xprevious: None,
            yprevious: None,
            hspeed: None,
            vspeed: None,
            speed: None,
            direction: None,
            gravity: None,
            friction: None,
            image_index: None,
            image_speed: None,
            image_xscale: None,
            image_yscale: None,
            depth: None,
            sprite_index: None,
            sprite: None,
            visible: None,
            solid: None,
            alarms: None,
            vars: None,
        }
    }

    /// A real-valued built-in by name (one of [`REAL_FIELDS`](Self::REAL_FIELDS)).
    pub fn real(&self, name: &str) -> Option<Real> {
        match name {
            "x" => self.x,
            "y" => self.y,
            "xprevious" => self.xprevious,
            "yprevious" => self.yprevious,
            "hspeed" => self.hspeed,
            "vspeed" => self.vspeed,
            "speed" => self.speed,
            "direction" => self.direction,
            "gravity" => self.gravity,
            "friction" => self.friction,
            "image_index" => self.image_index,
            "image_speed" => self.image_speed,
            "image_xscale" => self.image_xscale,
            "image_yscale" => self.image_yscale,
            "depth" => self.depth,
            _ => None,
        }
    }

    /// Mutable access to a real-valued built-in; `None` if `name` isn't one.
    pub fn real_mut(&mut self, name: &str) -> Option<&mut Option<Real>> {
        Some(match name {
            "x" => &mut self.x,
            "y" => &mut self.y,
            "xprevious" => &mut self.xprevious,
            "yprevious" => &mut self.yprevious,
            "hspeed" => &mut self.hspeed,
            "vspeed" => &mut self.vspeed,
            "speed" => &mut self.speed,
            "direction" => &mut self.direction,
            "gravity" => &mut self.gravity,
            "friction" => &mut self.friction,
            "image_index" => &mut self.image_index,
            "image_speed" => &mut self.image_speed,
            "image_xscale" => &mut self.image_xscale,
            "image_yscale" => &mut self.image_yscale,
            "depth" => &mut self.depth,
            _ => return None,
        })
    }

    /// `alarm[i]`, or `None` if the alarms weren't dumped. A missing entry is -1.
    pub fn alarm(&self, i: u32) -> Option<i32> {
        self.alarms
            .as_ref()
            .map(|a| a.get(&i).copied().unwrap_or(-1))
    }
}

/// Writes a trace: an optional header, then one line per frame with the filter applied and instances sorted
/// by id.
pub struct TraceWriter<W: Write> {
    out: W,
    filter: Filter,
}

impl<W: Write> TraceWriter<W> {
    /// Without a header the output is exactly the oracle's shape.
    pub fn new(out: W, filter: Filter) -> Self {
        TraceWriter { out, filter }
    }

    /// Writes a header line first, recording the filter (if any) in it.
    pub fn with_header(mut out: W, mut header: TraceHeader, filter: Filter) -> io::Result<Self> {
        header.filter = (!filter.is_empty()).then(|| filter.clone());
        serde_json::to_writer(&mut out, &header).map_err(io::Error::other)?;
        out.write_all(b"\n")?;
        Ok(TraceWriter { out, filter })
    }

    pub fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        let sorted = |list: &[Instance]| list.is_sorted_by_key(|i| i.id);
        let needs_work = !self.filter.is_empty()
            || !sorted(&frame.instances)
            || frame.deactivated.as_deref().is_some_and(|d| !sorted(d));
        if needs_work {
            let mut f = frame.clone();
            self.filter.apply(&mut f);
            f.instances.sort_by_key(|i| i.id);
            if let Some(d) = &mut f.deactivated {
                d.sort_by_key(|i| i.id);
            }
            serde_json::to_writer(&mut self.out, &f)
        } else {
            serde_json::to_writer(&mut self.out, frame)
        }
        .map_err(io::Error::other)?;
        self.out.write_all(b"\n")
    }

    pub fn into_inner(self) -> W {
        self.out
    }
}

/// Reads a trace frame by frame, so traces larger than memory can be diffed.
///
/// Besides the JSON, it checks that frame numbers strictly increase, that instance ids are unique within a
/// frame (across active and deactivated), and that alarm indices are 0-11. After the first error the
/// iterator ends.
pub struct TraceReader<R: BufRead> {
    input: R,
    header: Option<TraceHeader>,
    /// A frame read while looking for the header.
    pending: Option<Frame>,
    line_no: u64,
    last_frame: Option<u64>,
    done: bool,
    buf: String,
}

impl<R: BufRead> TraceReader<R> {
    /// Reads the first line, which is either a header or the first frame.
    pub fn new(input: R) -> Result<Self, Error> {
        let mut reader = TraceReader {
            input,
            header: None,
            pending: None,
            line_no: 0,
            last_frame: None,
            done: false,
            buf: String::new(),
        };
        let Some(line_no) = reader.next_line()? else {
            return Ok(reader);
        };
        let value: serde_json::Value = serde_json::from_str(reader.buf.trim())
            .map_err(|e| Error::at(line_no, format!("invalid JSON: {e}")))?;
        if !value.is_object() {
            return Err(Error::at(line_no, "invalid frame: expected a JSON object"));
        }
        if value.get("format").is_some() {
            let header = TraceHeader::deserialize(value)
                .map_err(|e| Error::at(line_no, format!("invalid trace header: {e}")))?;
            header.validate().map_err(|e| Error::at(line_no, e))?;
            reader.header = Some(header);
        } else {
            let frame = Frame::deserialize(value)
                .map_err(|e| Error::at(line_no, format!("invalid frame: {e}")))?;
            reader.pending = Some(reader.check(frame, line_no)?);
        }
        Ok(reader)
    }

    /// The header line, if the trace has one.
    pub fn header(&self) -> Option<&TraceHeader> {
        self.header.as_ref()
    }

    /// Reads the next non-blank line into `buf`; returns its number.
    fn next_line(&mut self) -> Result<Option<u64>, Error> {
        loop {
            self.buf.clear();
            let n = self
                .input
                .read_line(&mut self.buf)
                .map_err(|e| Error::at(self.line_no + 1, e.to_string()))?;
            if n == 0 {
                return Ok(None);
            }
            self.line_no += 1;
            if !self.buf.trim().is_empty() {
                return Ok(Some(self.line_no));
            }
        }
    }

    fn check(&mut self, frame: Frame, line: u64) -> Result<Frame, Error> {
        let n = frame.frame;
        if let Some(last) = self.last_frame
            && n <= last
        {
            return Err(Error::at(
                line,
                format!("frame {n} after frame {last}; frames must increase"),
            ));
        }
        let all = frame
            .instances
            .iter()
            .chain(frame.deactivated.iter().flatten());
        let mut ids = HashSet::new();
        for inst in all {
            if !ids.insert(inst.id) {
                return Err(Error::at(
                    line,
                    format!("frame {n}: instance id {} appears twice", inst.id),
                ));
            }
            if let Some(bad) = inst
                .alarms
                .iter()
                .flat_map(|a| a.keys())
                .find(|&&k| k >= ALARMS)
            {
                return Err(Error::at(
                    line,
                    format!("frame {n}: instance {} has alarm {bad} (0-11)", inst.id),
                ));
            }
        }
        self.last_frame = Some(n);
        Ok(frame)
    }

    fn read_frame(&mut self) -> Result<Option<Frame>, Error> {
        if let Some(f) = self.pending.take() {
            return Ok(Some(f));
        }
        let Some(line) = self.next_line()? else {
            return Ok(None);
        };
        let text = self.buf.trim();
        // serde would also accept a struct written as a JSON list.
        if !text.starts_with('{') {
            return Err(Error::at(line, "invalid frame: expected a JSON object"));
        }
        let frame: Frame = serde_json::from_str(text)
            .map_err(|e| Error::at(line, format!("invalid frame: {e}")))?;
        self.check(frame, line).map(Some)
    }
}

impl<R: BufRead> Iterator for TraceReader<R> {
    type Item = Result<Frame, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let result = self.read_frame().transpose();
        if !matches!(result, Some(Ok(_))) {
            self.done = true;
        }
        result
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::value::Scalar;

    /// A fully populated instance.
    pub fn instance(id: i32, object: &str, x: f64) -> Instance {
        let mut i = Instance::new(id, object);
        i.object_index = Some(id % 7);
        for (n, name) in Instance::REAL_FIELDS.iter().enumerate() {
            *i.real_mut(name).unwrap() = Some(Real(n as f64 * 0.5));
        }
        i.x = Some(Real(x));
        i.sprite_index = Some(3);
        i.sprite = Some(format!("s{}", &object[1..]));
        i.visible = Some(true);
        i.solid = Some(false);
        i.alarms = Some(BTreeMap::from([(0, 5), (8, -1)]));
        i.vars = Some(BTreeMap::from([
            ("xVel".into(), Value::from(0.1 + 0.2)),
            ("name".into(), Value::from("bob")),
        ]));
        i
    }

    pub fn frame(n: u64, instances: Vec<Instance>) -> Frame {
        Frame {
            frame: n,
            room: Some(3),
            room_name: Some("rLevel".into()),
            seed: Some(12345 + n as i32),
            instance_count: Some(instances.len() as u64),
            instances,
            deactivated: None,
            globals: Some(BTreeMap::from([("money".into(), Value::from(500.0))])),
            builtins: None,
        }
    }

    fn write(header: Option<TraceHeader>, filter: Filter, frames: &[Frame]) -> String {
        let mut w = match header {
            Some(h) => TraceWriter::with_header(Vec::new(), h, filter).unwrap(),
            None => TraceWriter::new(Vec::new(), filter),
        };
        for f in frames {
            w.write_frame(f).unwrap();
        }
        String::from_utf8(w.into_inner()).unwrap()
    }

    fn read(text: &str) -> Result<(Option<TraceHeader>, Vec<Frame>), Error> {
        let reader = TraceReader::new(text.as_bytes())?;
        let header = reader.header().cloned();
        Ok((header, reader.collect::<Result<_, _>>()?))
    }

    #[test]
    fn round_trip() {
        let mut weird = instance(100002, "oBat", f64::NAN);
        weird.sprite = Some(String::new());
        weird.sprite_index = Some(-1);
        weird.y = Some(Real(-0.0));
        weird.depth = Some(Real(f64::NEG_INFINITY));
        let arr = BTreeMap::from([
            (3, Scalar::Real(Real(1e-300))),
            (64000, Scalar::Str("x".into())),
        ]);
        weird
            .vars
            .as_mut()
            .unwrap()
            .insert("grid".into(), Value::Array(arr));
        weird
            .vars
            .as_mut()
            .unwrap()
            .insert("inf".into(), Value::from(f64::INFINITY));
        let mut f0 = frame(0, vec![instance(100001, "oPlayer1", 1.0 / 3.0), weird]);
        f0.deactivated = Some(vec![instance(99, "oBrick", 4.0)]);
        f0.builtins = Some(BTreeMap::from([(
            "view_xview[0]".into(),
            Value::from(16.0),
        )]));
        let frames = vec![
            f0,
            frame(5, vec![]),
            Frame {
                frame: 6,
                ..Frame::default()
            },
        ];
        for header in [None, Some(TraceHeader::new(Some("port")))] {
            let text = write(header.clone(), Filter::default(), &frames);
            let (h, back) = read(&text).unwrap();
            assert_eq!(h, header);
            assert_eq!(back, frames);
            // Bit-exact, not just PartialEq-equal (which treats NaNs alike).
            assert_eq!(
                back[0].instances[1].y.unwrap().0.to_bits(),
                (-0.0f64).to_bits()
            );
            assert!(text.ends_with("{\"frame\":6,\"instances\":[]}\n"));
        }
    }

    #[test]
    fn writes_the_oracle_shape() {
        let mut i = Instance::new(133637, "oBrick");
        i.object_index = Some(320);
        i.x = Some(Real(80.0));
        i.sprite_index = Some(731);
        i.sprite = Some("sBrickDown".into());
        i.alarms = Some(BTreeMap::new());
        let f = Frame {
            frame: 3,
            room: Some(100),
            room_name: Some("rIntro".into()),
            seed: Some(-5),
            instance_count: Some(1),
            instances: vec![i],
            ..Frame::default()
        };
        assert_eq!(
            write(None, Filter::default(), &[f]),
            "{\"frame\":3,\"room\":100,\"room_name\":\"rIntro\",\"seed\":-5,\"instance_count\":1,\"instances\":\
             [{\"id\":133637,\"object_index\":320,\"object\":\"oBrick\",\"x\":80.0,\"sprite_index\":731,\
             \"sprite\":\"sBrickDown\",\"alarms\":{}}]}\n"
        );
    }

    #[test]
    fn reads_oracle_lines() {
        // Trimmed from the oracle's start.jsonl and sample_fields_frame1500.jsonl.
        let text = include_str!("../tests/fixtures/oracle_frames.jsonl");
        let (header, frames) = read(text).unwrap();
        assert!(header.is_none());
        assert_eq!(frames.len(), 3);
        let f = &frames[2];
        assert_eq!(
            (f.frame, f.room, f.room_name.as_deref(), f.seed),
            (1500, Some(114), Some("rTutorial"), Some(353348486))
        );
        let player = f.instances.iter().find(|i| i.object == "oPlayer1").unwrap();
        assert_eq!(player.image_xscale, Some(Real(-1.0)));
        let vars = player.vars.as_ref().unwrap();
        assert_eq!(vars["climbAcc"], Value::from(0.6000000000000001));
        assert_eq!(vars["holdItemType"], Value::from(""));
        let globals = f.globals.as_ref().unwrap();
        assert_eq!(
            globals["roomPath"].to_string().split(", ").nth(5),
            Some("[1,0]=0")
        );
        // Re-serializing gives back the oracle's bytes, numbers included.
        let original: Vec<&str> = text.lines().collect();
        for (f, line) in frames.iter().zip(&original) {
            assert_eq!(serde_json::to_string(f).unwrap(), *line);
        }
    }

    #[test]
    fn writer_filters_and_sorts() {
        let filter = Filter {
            objects: vec!["oPlayer*".into()],
            fields: vec!["x".into(), "vars.xVel".into(), "alarms[8]".into()],
            ignore_fields: vec!["seed".into()],
            ..Filter::default()
        };
        let mut f = frame(
            0,
            vec![
                instance(100003, "oPlayer2", 4.0),
                instance(100002, "oBat", 1.0),
                instance(100001, "oPlayer1", 2.0),
            ],
        );
        f.deactivated = Some(vec![instance(7, "oPlayer9", 0.0), instance(6, "oBat", 0.0)]);
        let header = TraceHeader::new(None);
        let text = write(Some(header.clone()), filter.clone(), &[f]);
        let (back_header, back) = read(&text).unwrap();
        assert_eq!(
            back_header,
            Some(TraceHeader {
                filter: Some(filter),
                ..header
            })
        );
        let f = &back[0];
        assert_eq!((f.seed, f.room, f.instance_count), (None, Some(3), Some(3)));
        assert!(f.globals.as_ref().unwrap().is_empty());
        assert_eq!(
            f.instances.iter().map(|i| i.id).collect::<Vec<_>>(),
            [100001, 100003]
        );
        assert_eq!(
            f.deactivated
                .as_ref()
                .unwrap()
                .iter()
                .map(|i| i.id)
                .collect::<Vec<_>>(),
            [7]
        );
        let i = &f.instances[0];
        assert_eq!(i.x, Some(Real(2.0)));
        assert_eq!(
            (i.y, i.sprite.as_ref(), i.visible, i.object_index),
            (None, None, None, None)
        );
        assert_eq!(i.alarms, Some(BTreeMap::from([(8, -1)])));
        assert_eq!(
            i.vars.as_ref().unwrap().keys().collect::<Vec<_>>(),
            ["xVel"]
        );

        // An empty filter isn't recorded.
        assert!(!write(Some(TraceHeader::new(None)), Filter::default(), &[]).contains("filter"));
    }

    #[test]
    fn malformed_traces_are_errors() {
        let h = r#"{"format":"spelunky-trace","version":1,"game":"spelunky-classic-1.1"}"#;
        let i =
            |extra: &str| format!(r#"{{"frame":0,"instances":[{{"id":1,"object":"a"{extra}}}]}}"#);
        let cases: Vec<(String, &str)> = vec![
            ("not json".into(), "line 1: invalid JSON"),
            ("[1]".into(), "line 1: invalid frame: expected a JSON object"),
            ("{\"frame\":0}\n[1]".into(), "line 2: invalid frame: expected a JSON object"),
            (h.replace("spelunky-trace", "spelunky-replay"), "format is"),
            (h.replace("\"version\":1", "\"version\":9"), "unsupported trace version 9"),
            (h.replace("1.1", "1.2"), "game is"),
            (h.replace('}', ",\"extra\":1}"), "line 1: invalid trace header: unknown field"),
            (format!("{h}\n{{\"frame\":0"), "line 2: invalid frame"),
            (format!("{h}\n{{\"frame\":-1}}"), "line 2: invalid frame"),
            (format!("{h}\n{{}}"), "missing field `frame`"),
            (format!("{h}\n\n{{\"frame\":0,\"bogus\":1}}"), "line 3: invalid frame: unknown field"),
            ("{\"frame\":3}\n{\"frame\":3}".into(), "line 2: frame 3 after frame 3"),
            ("{\"frame\":0,\"seed\":2147483648}".into(), "line 1: invalid frame"),
            ("{\"frame\":0,\"seed\":1.5}".into(), "invalid frame"),
            ("{\"frame\":0,\"instances\":[{\"id\":1}]}".into(), "missing field `object`"),
            (
                r#"{"frame":0,"instances":[{"id":1,"object":"a"}],"deactivated":[{"id":1,"object":"b"}]}"#.into(),
                "instance id 1 appears twice",
            ),
            (i(r#","alarms":{"12":1}"#), "has alarm 12 (0-11)"),
            (i(r#","alarms":{"x":1}"#), "invalid frame"),
            (i(r#","alarms":{"0":1.5}"#), "invalid frame"),
            (i(r#","x":"5""#), "expected a real"),
            (i(r#","sprite":3"#), "invalid frame"),
            (i(r#","vars":{"a":null}"#), "invalid frame"),
            (i(r#","vars":{"a":[1]}"#), "JSON lists are not values"),
            ("{\"frame\":0,\"globals\":{\"a\":{\"0\":{}}}}".into(), "an array is not allowed here"),
        ];
        for (text, want) in cases {
            let err = read(&text).expect_err(&text).to_string();
            assert!(err.contains(want), "{text:?}: got {err:?}, want {want:?}");
        }
    }

    #[test]
    fn empty_and_blank_traces() {
        assert_eq!(read("").unwrap(), (None, vec![]));
        assert_eq!(read("\n \n").unwrap(), (None, vec![]));
    }

    #[test]
    fn reader_stops_after_an_error() {
        let text = "{\"frame\":0}\ngarbage\n{\"frame\":2}\n";
        let results: Vec<_> = TraceReader::new(text.as_bytes()).unwrap().collect();
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok() && results[1].is_err());
    }
}
