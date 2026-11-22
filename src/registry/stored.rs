use std::fmt::Display;
use tracing::warn;

#[derive(Debug, PartialEq)]
pub enum Outcome<E> {
    Created,
    Kept,
    Differs,
    Legacy,
    Unreadable(E),
}

pub fn choose<T: Clone + PartialEq, E>(stored: Option<Result<T, E>>, is_new: bool, configured: &T, vanilla: T) -> (T, Outcome<E>) {
    match (stored, is_new) {
        (Some(Ok(value)), _) if value == *configured => (value, Outcome::Kept),
        (Some(Ok(value)), _) => (value, Outcome::Differs),
        (Some(Err(err)), _) => (vanilla, Outcome::Unreadable(err)),
        (None, true) => (configured.clone(), Outcome::Created),
        (None, false) => (vanilla, Outcome::Legacy),
    }
}

pub fn report<E: Display>(world: &str, what: &str, outcome: &Outcome<E>) {
    match outcome {
        Outcome::Created | Outcome::Kept => {}
        Outcome::Differs => warn!("\"{world}\" keeps the {what} it was created with, which differ from the config"),
        Outcome::Legacy => warn!("\"{world}\" was created before its {what} were stored, it keeps the defaults"),
        Outcome::Unreadable(err) => warn!("the {what} of \"{world}\" are unreadable ({err}), using the defaults"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Chosen = (u32, Outcome<String>);

    fn pick(stored: Option<Result<u32, String>>, is_new: bool, configured: u32) -> Chosen {
        choose(stored, is_new, &configured, 0)
    }

    #[test]
    fn a_new_world_takes_the_configured_value() {
        assert_eq!(pick(None, true, 7), (7, Outcome::Created));
    }

    #[test]
    fn the_stored_value_wins_over_the_config() {
        assert_eq!(pick(Some(Ok(3)), false, 7), (3, Outcome::Differs));
        assert_eq!(pick(Some(Ok(7)), false, 7), (7, Outcome::Kept));
    }

    #[test]
    fn a_world_from_before_the_value_existed_keeps_the_default() {
        assert_eq!(pick(None, false, 7), (0, Outcome::Legacy));
    }

    #[test]
    fn an_unreadable_stored_value_falls_back_to_the_default() {
        assert_eq!(pick(Some(Err("bad".into())), false, 7), (0, Outcome::Unreadable("bad".into())));
    }
}
