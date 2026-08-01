use crate::{
    error::{Expected, Reason, SyntaxError},
    parse::Parser,
};

/// Parsing implementation for strings.
impl<'src> Parser<'src> {
    /// Parse a string (including quotes), verifying that the input is exhausted.
    pub(super) const fn string(&mut self) -> Result<&'src str, SyntaxError> {
        match self.next_non_ws() {
            Some(b'"') => {
                _ = self.match_advance();
                match self.string_content() {
                    Ok(content) => {
                        if self.next_non_ws().is_none() {
                            Ok(content)
                        } else {
                            Err(self.error(Reason::String, &[]))
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Some(_) | None => Err(self.error(Reason::String, &[Expected::Punctuation('"')])),
        }
    }

    /// Parse the contents of a string, not including the opening quote.
    const fn string_content(&mut self) -> Result<&'src str, SyntaxError> {
        loop {
            match self.next() {
                // escape
                Some(b'\\') => break self.string_escape(),
                // end of string
                Some(b'"') => {
                    let matched = self.match_advance();
                    let (content, _) = matched.split_at(matched.len() - 1);
                    break Ok(content);
                }
                // illegal cases
                None | Some(0..=0x1F) => break Err(self.error(Reason::String, &[Expected::String])),
                // normal string character(-part)
                Some(_) => continue,
            }
        }
    }

    /// Parse a single escape sequence and then the rest of a string.
    const fn string_escape(&mut self) -> Result<&'src str, SyntaxError> {
        match self.next() {
            // Unicode escape
            Some(b'u') => self.unicode_escape(),
            // simple escapes
            Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.string_content(),
            Some(_) | None => {
                Err(self.error(Reason::String, &[Expected::Escape, Expected::UnicodeEscape]))
            }
        }
    }

    /// Parse a Unicode escape sequence and then the rest of a string.
    const fn unicode_escape(&mut self) -> Result<&'src str, SyntaxError> {
        let mut times = 0;
        while times < 4 {
            times += 1;
            match self.next() {
                Some(b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f') => continue,
                Some(_) | None => {
                    return Err(self.error(Reason::String, &[Expected::UnicodeEscape]));
                }
            }
        }
        self.string_content()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(Parser::new(r#""""#).string(), Ok(""));
    }

    #[test]
    fn ascii() {
        assert_eq!(Parser::new(r#""hello world""#).string(), Ok("hello world"));
    }

    #[test]
    fn whitespace() {
        assert_eq!(
            Parser::new(r#"   "hello world"   "#).string(),
            Ok("hello world")
        );
    }

    #[test]
    fn simple_escapes() {
        assert_eq!(
            Parser::new(r#""\r\n\\\/\"\b\f\t""#).string(),
            Ok(r#"\r\n\\\/\"\b\f\t"#)
        );
    }

    #[test]
    fn unicode_escapes() {
        assert_eq!(
            Parser::new(r#""\uD800\uabcd\u1234""#).string(),
            Ok(r"\uD800\uabcd\u1234")
        );
    }

    #[test]
    fn complex() {
        assert_eq!(
            Parser::new(r#""this is \\\" a \u000a \t reall/ \\""#).string(),
            Ok(r#"this is \\\" a \u000a \t reall/ \\"#)
        );
    }
}
