use crate::command::parameter::{ArgumentType, CommandParameter, ParameterKind};
use crate::registry::command_registry::CommandRegistry;

#[derive(Default)]
pub struct Completions {
    commands: Vec<Command>,
}

struct Command {
    names: Vec<String>,
    overloads: Vec<Vec<Param>>,
}

struct Param {
    values: Option<Vec<String>>,
    width: Option<usize>,
}

#[derive(Default)]
pub struct Analysis {
    pub start: usize,
    pub candidates: Vec<String>,
}

impl Param {
    fn new(parameter: &CommandParameter, registry: &CommandRegistry, command_names: &[String]) -> Self {
        let values = match &parameter.kind {
            ParameterKind::Enum(values) => Some(values.values.iter().map(|value| value.to_string()).collect()),
            ParameterKind::Literal(word) => Some(vec![word.to_string()]),
            ParameterKind::SoftEnum(name) => Some(registry.soft_enum(name).map(<[String]>::to_vec).unwrap_or_default()),
            ParameterKind::CommandName => Some(command_names.to_vec()),
            ParameterKind::Argument(_) | ParameterKind::Postfix(_) => None,
        };
        let width = match &parameter.kind {
            ParameterKind::Argument(ArgumentType::Position | ArgumentType::BlockPosition) => Some(3),
            ParameterKind::Argument(ArgumentType::Message | ArgumentType::RawText | ArgumentType::Json | ArgumentType::Command) => None,
            _ => Some(1),
        };
        Self { values, width }
    }

    fn accepts(&self, token: &str, partial: bool) -> bool {
        let token = token.to_ascii_lowercase();
        self.values.as_ref().is_none_or(|values| {
            values
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .any(|value| if partial { value.starts_with(&token) } else { value == token })
        })
    }
}

impl Completions {
    pub fn build(registry: &CommandRegistry) -> Self {
        let command_names: Vec<String> = registry.commands().map(|command| command.name.to_string()).collect();
        let commands = registry
            .commands()
            .map(|command| Command {
                names: std::iter::once(command.name.to_string()).chain(command.aliases.iter().map(|alias| alias.to_string())).collect(),
                overloads: command
                    .overloads
                    .iter()
                    .map(|overload| overload.parameters.iter().map(|parameter| Param::new(parameter, registry, &command_names)).collect())
                    .collect(),
            })
            .collect();
        Self { commands }
    }

    pub fn analyze(&self, line: &str) -> Analysis {
        let body = line.trim_start().trim_start_matches('/');
        let offset = line.len() - body.len();
        let mut tokens: Vec<(usize, &str)> = Vec::new();
        let mut start = None;
        for (index, char) in body.char_indices() {
            match (char.is_whitespace(), start) {
                (true, Some(begin)) => {
                    tokens.push((begin, &body[begin..index]));
                    start = None;
                }
                (false, None) => start = Some(index),
                _ => {}
            }
        }
        let current = match start {
            Some(begin) => (begin, &body[begin..]),
            None => (body.len(), ""),
        };
        let mut analysis = Analysis {
            start: line[..offset + current.0].chars().count(),
            ..Default::default()
        };
        let prefix = current.1.to_ascii_lowercase();

        let Some(&(_, name)) = tokens.first() else {
            analysis.candidates = self
                .commands
                .iter()
                .flat_map(|command| &command.names)
                .filter(|name| name.to_ascii_lowercase().starts_with(&prefix))
                .cloned()
                .collect();
            analysis.candidates.sort();
            analysis.candidates.dedup();
            return analysis;
        };
        let Some(command) = self.commands.iter().find(|command| command.names.iter().any(|known| known.eq_ignore_ascii_case(name))) else {
            return analysis;
        };

        let arguments = &tokens[1..];
        for overload in &command.overloads {
            let Some(position) = Self::current_parameter(overload, arguments, current.1) else { continue };
            if let Some(values) = overload.get(position).and_then(|param| param.values.as_ref()) {
                analysis.candidates.extend(values.iter().filter(|value| value.to_ascii_lowercase().starts_with(&prefix)).cloned());
            }
        }
        analysis.candidates.sort();
        analysis.candidates.dedup();
        analysis
    }

    fn current_parameter(overload: &[Param], arguments: &[(usize, &str)], partial: &str) -> Option<usize> {
        let mut remaining = arguments;
        for (index, param) in overload.iter().enumerate() {
            let Some(width) = param.width else { return Some(index) };
            if remaining.len() < width {
                let accepted = remaining.iter().all(|(_, token)| param.accepts(token, false)) && param.accepts(partial, true);
                return accepted.then_some(index);
            }
            if !remaining[..width].iter().all(|(_, token)| param.accepts(token, false)) {
                return None;
            }
            remaining = &remaining[width..];
        }
        (remaining.is_empty() && partial.is_empty()).then_some(overload.len())
    }
}
