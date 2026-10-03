pub struct Identifier;

impl Identifier {
    pub fn validate(id: &str) -> Result<(), String> {
        let Some((namespace, path)) = id.split_once(':') else {
            return Err(format!("identifier {:?} doesn't contain ':'", id));
        };

        if let Some(invalid) = namespace.chars().find(|&c| !Self::is_namespace_char_valid(c)) {
            return Err(format!(
                "invalid namespace {:?}: character {:?} is not allowed. \
                must only contain lowercase letters, digits, '_', '-', and '.'",
                namespace, invalid
            ));
        }

        if let Some(invalid) = path.chars().find(|&c| !Self::is_path_char_valid(c)) {
            return Err(format!(
                "invalid path {:?}: character {:?} is not allowed. \
                must only contain lowercase letters, digits, '_', '-', '.', and '/'",
                path, invalid
            ));
        }

        Ok(())
    }

    #[inline]
    fn is_path_char_valid(c: char) -> bool {
        matches!(c,
            'a'..='z'
            | '0'..='9'
            | '_'
            | '-'
            | '.'
            | '/'
        )
    }

    #[inline]
    fn is_namespace_char_valid(c: char) -> bool {
        matches!(c,
            'a'..='z'
            | '0'..='9'
            | '_'
            | '-'
            | '.'
        )
    }
}
