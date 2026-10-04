use crate::command::parameter::{ArgumentType, CommandOverload, CommandParameter, ParameterKind};
use chorus_core::permission::PermissionLevel;
use glam::{DVec3, IVec3, Vec2};
use std::collections::HashMap;

/// One component of a position: a plain number, relative to the sender (`~`), or along the sender's
/// facing (`^`, left/up/forward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coordinate {
    Absolute(f64),
    Relative(f64),
    Local(f64),
}

impl Coordinate {
    fn parse(text: &str) -> Option<Self> {
        let number = |rest: &str| if rest.is_empty() { Some(0.0) } else { rest.parse::<f64>().ok().filter(|value| value.is_finite()) };
        if let Some(rest) = text.strip_prefix('~') {
            Some(Self::Relative(number(rest)?))
        } else if let Some(rest) = text.strip_prefix('^') {
            Some(Self::Local(number(rest)?))
        } else {
            text.parse::<f64>().ok().filter(|value| value.is_finite()).map(Self::Absolute)
        }
    }

    /// Resolves an absolute or relative coordinate against `origin`; local coordinates need a whole
    /// [`CommandPosition`].
    pub fn resolve(self, origin: f64) -> f64 {
        match self {
            Self::Absolute(value) => value,
            Self::Relative(offset) | Self::Local(offset) => origin + offset,
        }
    }
}

/// A position argument. Use [`resolve`](Self::resolve) to turn it into world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandPosition {
    pub x: Coordinate,
    pub y: Coordinate,
    pub z: Coordinate,
}

impl CommandPosition {
    pub fn is_local(&self) -> bool {
        matches!(self.x, Coordinate::Local(_))
    }

    /// World coordinates relative to `origin`, facing `rotation` (pitch, yaw in degrees, as in
    /// `Transform::rotation`) for `^`.
    pub fn resolve(&self, origin: DVec3, rotation: Vec2) -> DVec3 {
        if let (Coordinate::Local(left), Coordinate::Local(up), Coordinate::Local(forward)) = (self.x, self.y, self.z) {
            let (pitch, yaw) = ((rotation.x as f64).to_radians(), (rotation.y as f64).to_radians());
            let forward_axis = DVec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos());
            let up_axis = DVec3::new(-yaw.sin() * -pitch.sin(), pitch.cos(), yaw.cos() * -pitch.sin());
            let left_axis = up_axis.cross(forward_axis);
            return origin + left_axis * left + up_axis * up + forward_axis * forward;
        }
        DVec3::new(self.x.resolve(origin.x), self.y.resolve(origin.y), self.z.resolve(origin.z))
    }

    /// The block containing [`resolve`](Self::resolve)'s result.
    pub fn resolve_block(&self, origin: DVec3, rotation: Vec2) -> IVec3 {
        self.resolve(origin, rotation).floor().as_ivec3()
    }
}

/// A parsed argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    Int(i32),
    Float(f32),
    Value(Coordinate),
    /// `None` for `*`.
    WildcardInt(Option<i32>),
    Range(Option<i32>, Option<i32>),
    Position(CommandPosition),
    Text(String),
}

/// The arguments of a command, parsed against the overload that matched.
#[derive(Debug, Clone, Default)]
pub struct CommandArgs {
    overload: usize,
    values: Vec<(String, ArgValue)>,
    raw: String,
}

impl CommandArgs {
    /// Index of the overload that matched, in the order the command declares them.
    pub fn overload(&self) -> usize {
        self.overload
    }

    /// Everything after the command name, as typed.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<&ArgValue> {
        self.values.iter().find(|(parameter, _)| parameter == name).map(|(_, value)| value)
    }

    pub fn int(&self, name: &str) -> Option<i32> {
        match self.get(name)? {
            ArgValue::Int(value) | ArgValue::WildcardInt(Some(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn float(&self, name: &str) -> Option<f32> {
        match self.get(name)? {
            ArgValue::Float(value) => Some(*value),
            ArgValue::Int(value) => Some(*value as f32),
            _ => None,
        }
    }

    pub fn value(&self, name: &str) -> Option<Coordinate> {
        match self.get(name)? {
            ArgValue::Value(value) => Some(*value),
            _ => None,
        }
    }

    /// `Some(None)` when the argument was `*`.
    pub fn wildcard_int(&self, name: &str) -> Option<Option<i32>> {
        match self.get(name)? {
            ArgValue::WildcardInt(value) => Some(*value),
            _ => None,
        }
    }

    pub fn range(&self, name: &str) -> Option<(Option<i32>, Option<i32>)> {
        match self.get(name)? {
            ArgValue::Range(min, max) => Some((*min, *max)),
            _ => None,
        }
    }

    pub fn position(&self, name: &str) -> Option<CommandPosition> {
        match self.get(name)? {
            ArgValue::Position(position) => Some(*position),
            _ => None,
        }
    }

    /// Text arguments: strings, targets, enum and soft enum values, literals, and rest-of-line text.
    pub fn string(&self, name: &str) -> Option<&str> {
        match self.get(name)? {
            ArgValue::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// Why no overload matched.
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    /// Usage of every overload that got as far as the one the message is about.
    pub usages: Vec<String>,
}

struct Token<'a> {
    text: String,
    start: usize,
    quoted: bool,
    source: &'a str,
}

/// Splits on whitespace, keeping `"quoted strings"` (with `\"` escapes) together.
fn tokenize(line: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some(&(start, char)) = chars.peek() {
        if char.is_whitespace() {
            chars.next();
            continue;
        }
        let mut text = String::new();
        let quoted = char == '"';
        if quoted {
            chars.next();
            while let Some((_, char)) = chars.next() {
                match char {
                    '\\' => text.extend(chars.next().map(|(_, escaped)| escaped)),
                    '"' => break,
                    _ => text.push(char),
                }
            }
        } else {
            while let Some(&(_, char)) = chars.peek() {
                if char.is_whitespace() {
                    break;
                }
                text.push(char);
                chars.next();
            }
        }
        tokens.push(Token { text, start, quoted, source: line });
    }
    tokens
}

/// Splits a token like `~~1~` into the coordinates it holds.
fn coordinate_parts(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (index, char) in text.char_indices().skip(1) {
        if char == '~' || char == '^' {
            parts.push(&text[start..index]);
            start = index;
        }
    }
    parts.push(&text[start..]);
    parts
}

struct Attempt {
    consumed: usize,
    message: String,
}

fn parse_overload(parameters: &[CommandParameter], tokens: &[Token], soft_enums: &HashMap<String, Vec<String>>, level: PermissionLevel) -> Result<Vec<(String, ArgValue)>, Attempt> {
    let mut values = Vec::with_capacity(parameters.len());
    let mut index = 0;
    for parameter in parameters {
        let Some(token) = tokens.get(index) else {
            if parameter.optional {
                break;
            }
            return Err(Attempt {
                consumed: index,
                message: format!("missing {}", parameter.usage_token()),
            });
        };
        let fail = |message: String| Attempt { consumed: index, message };
        let text = token.text.as_str();
        let value = match &parameter.kind {
            ParameterKind::Literal(word) => {
                if !text.eq_ignore_ascii_case(word) {
                    return Err(fail(format!("expected \"{word}\" but got \"{text}\"")));
                }
                ArgValue::Text(word.to_string())
            }
            ParameterKind::Enum(values) => match values.find(text) {
                Some(value) if values.constraints_of(value).required_level() > level => {
                    return Err(fail(format!("you don't have permission to use \"{value}\"")));
                }
                Some(value) => ArgValue::Text(value.to_owned()),
                None => return Err(fail(format!("\"{text}\" is not one of {}", values.values.join(", ")))),
            },
            ParameterKind::SoftEnum(name) => {
                let known = soft_enums.get(name.as_ref());
                match known.and_then(|values| values.iter().find(|value| value.eq_ignore_ascii_case(text))) {
                    Some(value) => ArgValue::Text(value.clone()),
                    None => return Err(fail(format!("\"{text}\" is not a valid {}", parameter.name))),
                }
            }
            ParameterKind::Postfix(postfix) => {
                let number = text.strip_suffix(postfix.as_ref()).or_else(|| text.strip_suffix(&postfix.to_ascii_lowercase())).unwrap_or(text);
                match number.parse() {
                    Ok(value) => ArgValue::Int(value),
                    Err(_) => return Err(fail(format!("expected a number followed by {postfix} but got \"{text}\""))),
                }
            }
            ParameterKind::Argument(kind) if kind.is_greedy() => {
                values.push((parameter.name.to_string(), ArgValue::Text(token.source[token.start..].trim_end().to_owned())));
                return Ok(values);
            }
            ParameterKind::Argument(ArgumentType::Position | ArgumentType::BlockPosition) => {
                let integer = matches!(parameter.kind, ParameterKind::Argument(ArgumentType::BlockPosition));
                let mut coordinates = Vec::with_capacity(3);
                let mut next = index;
                while coordinates.len() < 3 {
                    let Some(token) = tokens.get(next).filter(|token| !token.quoted) else { break };
                    let parts = coordinate_parts(&token.text);
                    if coordinates.len() + parts.len() > 3 {
                        break;
                    }
                    for part in parts {
                        let coordinate = Coordinate::parse(part).filter(|coordinate| !integer || !matches!(coordinate, Coordinate::Absolute(value) if value.fract() != 0.0));
                        match coordinate {
                            Some(coordinate) => coordinates.push(coordinate),
                            None => {
                                return Err(Attempt {
                                    consumed: next,
                                    message: format!("\"{part}\" is not a valid coordinate"),
                                });
                            }
                        }
                    }
                    next += 1;
                }
                let [x, y, z] = coordinates[..] else {
                    return Err(fail(format!("expected three coordinates for {}", parameter.usage_token())));
                };
                let local = [x, y, z].iter().filter(|coordinate| matches!(coordinate, Coordinate::Local(_))).count();
                if local != 0 && local != 3 {
                    return Err(fail("local coordinates (^) can't be mixed with other kinds".to_owned()));
                }
                values.push((parameter.name.to_string(), ArgValue::Position(CommandPosition { x, y, z })));
                index = next;
                continue;
            }
            ParameterKind::Argument(kind) => match parse_argument(*kind, text) {
                Some(value) => value,
                None => return Err(fail(format!("\"{text}\" is not a valid {}", kind.label()))),
            },
        };
        values.push((parameter.name.to_string(), value));
        index += 1;
    }
    match tokens.get(index) {
        Some(extra) => Err(Attempt {
            consumed: index,
            message: format!("unexpected \"{}\"", extra.text),
        }),
        None => Ok(values),
    }
}

fn parse_argument(kind: ArgumentType, text: &str) -> Option<ArgValue> {
    Some(match kind {
        ArgumentType::Int => ArgValue::Int(text.parse().ok()?),
        ArgumentType::Float => ArgValue::Float(text.parse().ok().filter(|value: &f32| value.is_finite())?),
        ArgumentType::Value => ArgValue::Value(Coordinate::parse(text).filter(|coordinate| !matches!(coordinate, Coordinate::Local(_)))?),
        ArgumentType::WildcardInt => ArgValue::WildcardInt(if text == "*" { None } else { Some(text.parse().ok()?) }),
        ArgumentType::Operator => {
            ["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><"].contains(&text).then_some(())?;
            ArgValue::Text(text.to_owned())
        }
        ArgumentType::CompareOperator => {
            ["<", "<=", "=", ">=", ">"].contains(&text).then_some(())?;
            ArgValue::Text(text.to_owned())
        }
        ArgumentType::IntegerRange => {
            let (min, max) = match text.split_once("..") {
                Some((min, max)) => (min, max),
                None => (text, text),
            };
            let bound = |bound: &str| if bound.is_empty() { Ok(None) } else { bound.parse().map(Some) };
            let (min, max) = (bound(min).ok()?, bound(max).ok()?);
            if min.is_none() && max.is_none() {
                return None;
            }
            ArgValue::Range(min, max)
        }
        ArgumentType::WildcardTarget | ArgumentType::Target | ArgumentType::String | ArgumentType::FilePath | ArgumentType::EquipmentSlot | ArgumentType::BlockStates => {
            ArgValue::Text(text.to_owned())
        }
        ArgumentType::BlockPosition | ArgumentType::Position | ArgumentType::Message | ArgumentType::RawText | ArgumentType::Json | ArgumentType::Command => return None,
    })
}

/// Parses `line` (everything after the command name) against each overload in order and returns the
/// first that fits, or the error from the overload that got furthest.
pub fn parse(name: &str, overloads: &[CommandOverload], line: &str, soft_enums: &HashMap<String, Vec<String>>, level: PermissionLevel) -> Result<CommandArgs, ParseError> {
    let tokens = tokenize(line);
    let empty = [CommandOverload::new(&[])];
    let overloads = if overloads.is_empty() { &empty[..] } else { overloads };

    let mut attempts = Vec::with_capacity(overloads.len());
    for (index, overload) in overloads.iter().enumerate() {
        match parse_overload(&overload.parameters, &tokens, soft_enums, level) {
            Ok(values) => {
                return Ok(CommandArgs {
                    overload: index,
                    values,
                    raw: line.trim().to_owned(),
                });
            }
            Err(attempt) => attempts.push((index, attempt)),
        }
    }

    let furthest = attempts.iter().map(|(_, attempt)| attempt.consumed).max().unwrap_or(0);
    let mut closest = attempts.into_iter().filter(|(_, attempt)| attempt.consumed == furthest);
    let (first, attempt) = closest.next().expect("there is at least one overload");
    let usages = std::iter::once(first)
        .chain(closest.map(|(index, _)| index))
        .map(|index| format!("/{name} {}", overloads[index].usage()).trim_end().to_owned())
        .collect();
    Err(ParseError { message: attempt.message, usages })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::parameter::{CommandEnum, CommandParameter};
    use crate::values;

    const MODES: CommandEnum = CommandEnum::new("Mode", values!["survival", "creative"]);
    const OVERLOADS: &[CommandOverload] = crate::overloads![
        [CommandParameter::literal("stop")],
        [CommandParameter::new("radius", ArgumentType::Int)],
        [CommandParameter::new("destination", ArgumentType::Position), CommandParameter::enumeration("mode", MODES).optional()],
        [
            CommandParameter::literal("say"),
            CommandParameter::new("name", ArgumentType::String),
            CommandParameter::new("message", ArgumentType::Message)
        ],
    ];

    fn parse_line(line: &str) -> Result<CommandArgs, ParseError> {
        parse("test", OVERLOADS, line, &HashMap::new(), PermissionLevel::Member)
    }

    #[test]
    fn picks_the_first_matching_overload() {
        assert_eq!(parse_line("stop").unwrap().overload(), 0);
        let args = parse_line("12").unwrap();
        assert_eq!((args.overload(), args.int("radius")), (1, Some(12)));
    }

    #[test]
    fn parses_compact_relative_positions_and_enums() {
        let args = parse_line("~ ~1.5~-2 CREATIVE").unwrap();
        assert_eq!(args.overload(), 2);
        let position = args.position("destination").unwrap();
        assert_eq!(position.resolve(DVec3::new(10.0, 64.0, 10.0), Vec2::ZERO), DVec3::new(10.0, 65.5, 8.0));
        assert_eq!(args.string("mode"), Some("creative"));
    }

    #[test]
    fn keeps_quoted_strings_and_greedy_text() {
        let args = parse_line("say \"some one\" hello   there world").unwrap();
        assert_eq!(args.overload(), 3);
        assert_eq!(args.string("name"), Some("some one"));
        assert_eq!(args.string("message"), Some("hello   there world"));
    }

    #[test]
    fn reports_the_overload_that_got_furthest() {
        let error = parse_line("1 2 3 adventure").unwrap_err();
        assert!(error.message.contains("adventure"), "{}", error.message);
        assert!(error.usages[0].contains("destination"), "{:?}", error.usages);
    }

    #[test]
    fn constrained_enum_values_need_the_permission() {
        use crate::command::parameter::EnumConstraints;
        const RESTRICTED: CommandEnum = CommandEnum::new("Restricted", values!["open", "locked"]).constrained(crate::constraints![("locked", EnumConstraints::OPERATOR_PERMISSIONS)]);
        const LOCKED: &[CommandOverload] = crate::overloads![[CommandParameter::enumeration("state", RESTRICTED)]];

        let run = |line: &str, level| parse("test", LOCKED, line, &HashMap::new(), level);
        assert!(run("open", PermissionLevel::Member).is_ok());
        assert!(run("locked", PermissionLevel::Member).unwrap_err().message.contains("permission"));
        assert_eq!(run("LOCKED", PermissionLevel::Operator).unwrap().string("state"), Some("locked"));
    }

    #[test]
    fn local_coordinates_follow_the_facing() {
        let args = parse_line("^ ^ ^2").unwrap();
        let position = args.position("destination").unwrap();
        let resolved = position.resolve(DVec3::ZERO, Vec2::new(0.0, 0.0));
        assert!((resolved - DVec3::new(0.0, 0.0, 2.0)).length() < 1e-9, "{resolved}");
        assert!(parse_line("^ ~ ^").is_err());
    }
}
