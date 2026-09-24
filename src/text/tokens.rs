//! A drawing's text, split into what its codes say.
//!
//! The model carries text with the codes the text itself is written in
//! (principles §6.1): TEXT, ATTRIB, ATTDEF and a dimension's text use percent
//! codes (`%%d` for a degree sign), and MTEXT adds its own backslash codes
//! (`\P` for a paragraph break, `\S1#2;` for stacked text). What each code
//! *means* is fixed by the format; what to do with it -- draw it, drop it,
//! compare text without it -- is each consumer's choice. So the splitting
//! lives here, once, and the choosing stays with the consumer: a renderer
//! draws a [`Token::Special`] as its sign, a search compares text by its
//! [`Token::Char`]s.
//!
//! The split is one pass: what one code produces is never read again as
//! another, and nothing of the text is lost -- a code this module does not
//! know, or one left unclosed, is a [`Token::Unknown`] carrying the text it
//! covers.

/// Which codes a string is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// TEXT, ATTRIB, ATTDEF, a dimension's text, a tolerance frame's text:
    /// percent codes only. A backslash is a character.
    Line,
    /// MTEXT: percent codes and MTEXT's backslash codes and `{}` groups.
    MText,
}

/// One piece of a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    /// A character of the text -- including one MTEXT writes escaped
    /// (`\\`, `\{`, `\}`) and a line feed (a tolerance frame's line break).
    Char(char),
    /// `%%d`, `%%p`, `%%c`: a sign the format names.
    Special(Special),
    /// `%%nnn`: the character with that three-digit code in the text's font.
    /// Which glyph that is depends on the font -- the standard shape fonts
    /// keep the degree, plus-minus and diameter signs at 127 to 129.
    CharCode(u16),
    /// `%%u` / `%%o`, and MTEXT's `\L` `\l` `\O` `\o` `\K` `\k`: a switch of
    /// how the text is drawn.
    Toggle(Toggle),
    /// MTEXT's `\P` (paragraph), `\N` (column), `\X` (a dimension text's
    /// split above and below its line).
    Break(Break),
    /// MTEXT's `\~`.
    NonBreakingSpace,
    /// MTEXT's `\S…;`: two parts stacked, with the separator the text wrote
    /// (`^` tolerance, `/` fraction bar, `#` diagonal bar).
    Stack {
        top: &'a str,
        bottom: &'a str,
        separator: char,
    },
    /// MTEXT's `\S…;` with no separator in it: the text as written.
    StackUnsplit(&'a str),
    /// An MTEXT code that sets a value until the next `;` -- `\A1;`,
    /// `\H2.5x;`, `\fArial|b1;` -- as its letter and its value.
    Property {
        code: char,
        value: &'a str,
    },
    /// MTEXT's `{` and `}`: a group that bounds what the codes inside set.
    GroupStart,
    GroupEnd,
    /// Anything that looks like a code but is not one this module reads, or
    /// is not closed: the text it covers, as written.
    Unknown(&'a str),
}

/// A sign a percent code names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special {
    /// `%%d`.
    Degree,
    /// `%%p`.
    PlusMinus,
    /// `%%c`.
    Diameter,
}

/// A drawing switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// `%%u` (flips), `\L` (on), `\l` (off).
    Underline(Switch),
    /// `%%o` (flips), `\O` (on), `\o` (off).
    Overline(Switch),
    /// `\K` (on), `\k` (off).
    StrikeThrough(Switch),
}

/// How a toggle sets its switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
    /// A percent code flips whatever the state is.
    Flip,
}

/// A line break MTEXT writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Break {
    Paragraph,
    Column,
    /// `\X`: in a dimension's text, what follows goes below the line.
    DimensionLine,
}

/// `text` split into its tokens, in order.
pub fn tokens(text: &str, kind: TextKind) -> Tokens<'_> {
    Tokens { text, at: 0, kind }
}

/// The iterator [`tokens`] returns.
#[derive(Debug, Clone)]
pub struct Tokens<'a> {
    text: &'a str,
    at: usize,
    kind: TextKind,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let rest = &self.text[self.at..];
        let c = rest.chars().next()?;
        let (token, len) = if rest.starts_with("%%") {
            percent(rest)
        } else if self.kind == TextKind::MText && c == '\\' {
            backslash(rest)
        } else if self.kind == TextKind::MText && c == '{' {
            (Token::GroupStart, 1)
        } else if self.kind == TextKind::MText && c == '}' {
            (Token::GroupEnd, 1)
        } else {
            (Token::Char(c), c.len_utf8())
        };
        self.at += len;
        Some(token)
    }
}

/// A percent code at the start of `rest` (which starts with `%%`), and its
/// length in bytes.
fn percent(rest: &str) -> (Token<'_>, usize) {
    let after = &rest[2..];
    let token = match after.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('d') => Token::Special(Special::Degree),
        Some('p') => Token::Special(Special::PlusMinus),
        Some('c') => Token::Special(Special::Diameter),
        Some('%') => Token::Char('%'),
        Some('u') => Token::Toggle(Toggle::Underline(Switch::Flip)),
        Some('o') => Token::Toggle(Toggle::Overline(Switch::Flip)),
        Some(d) if d.is_ascii_digit() => {
            let digits = after
                .get(..3)
                .filter(|d| d.bytes().all(|b| b.is_ascii_digit()));
            return match digits.and_then(|d| d.parse().ok()) {
                Some(code) => (Token::CharCode(code), 5),
                None => (Token::Unknown("%%"), 2),
            };
        }
        _ => return (Token::Unknown("%%"), 2),
    };
    (token, 3)
}

/// An MTEXT backslash code at the start of `rest` (which starts with `\`),
/// and its length in bytes.
fn backslash(rest: &str) -> (Token<'_>, usize) {
    let Some(c) = rest[1..].chars().next() else {
        return (Token::Unknown("\\"), 1);
    };
    let simple = match c {
        'P' => Some(Token::Break(Break::Paragraph)),
        'N' => Some(Token::Break(Break::Column)),
        'X' => Some(Token::Break(Break::DimensionLine)),
        '~' => Some(Token::NonBreakingSpace),
        '\\' | '{' | '}' => Some(Token::Char(c)),
        'L' => Some(Token::Toggle(Toggle::Underline(Switch::On))),
        'l' => Some(Token::Toggle(Toggle::Underline(Switch::Off))),
        'O' => Some(Token::Toggle(Toggle::Overline(Switch::On))),
        'o' => Some(Token::Toggle(Toggle::Overline(Switch::Off))),
        'K' => Some(Token::Toggle(Toggle::StrikeThrough(Switch::On))),
        'k' => Some(Token::Toggle(Toggle::StrikeThrough(Switch::Off))),
        _ => None,
    };
    if let Some(token) = simple {
        return (token, 1 + c.len_utf8());
    }
    let body_at = 1 + c.len_utf8();
    match c {
        'S' | 'A' | 'C' | 'c' | 'F' | 'f' | 'H' | 'Q' | 'T' | 'W' | 'p' => {
            // The value runs to the next `;`; without one the code is not
            // closed, and it is text.
            let Some(end) = rest[body_at..].find(';') else {
                return (Token::Unknown(&rest[..body_at]), body_at);
            };
            let value = &rest[body_at..body_at + end];
            let len = body_at + end + 1;
            if c != 'S' {
                return (Token::Property { code: c, value }, len);
            }
            match value.find(['^', '/', '#']) {
                Some(at) => (
                    Token::Stack {
                        top: &value[..at],
                        bottom: &value[at + 1..],
                        separator: value[at..].chars().next().unwrap_or('/'),
                    },
                    len,
                ),
                None => (Token::StackUnsplit(value), len),
            }
        }
        _ => (Token::Unknown(&rest[..body_at]), body_at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(text: &str, kind: TextKind) -> Vec<Token<'_>> {
        tokens(text, kind).collect()
    }

    #[test]
    fn percent_codes_in_any_text() {
        use Token::*;
        assert_eq!(
            all("%%c32 %%D%%p%%%x", TextKind::Line),
            [
                Special(super::Special::Diameter),
                Char('3'),
                Char('2'),
                Char(' '),
                Special(super::Special::Degree),
                Special(super::Special::PlusMinus),
                Char('%'),
                Char('x'),
            ]
        );
        assert_eq!(all("90%%127", TextKind::Line)[2], CharCode(127));
        assert_eq!(
            all("%%uA", TextKind::Line)[0],
            Toggle(super::Toggle::Underline(Switch::Flip))
        );
        // Not a code: kept.
        assert_eq!(all("%%12", TextKind::Line)[0], Unknown("%%"));
        assert_eq!(all("%%z", TextKind::Line)[0], Unknown("%%"));
    }

    #[test]
    fn outside_mtext_a_backslash_is_a_character() {
        assert_eq!(
            all(r"\P{x}", TextKind::Line),
            r"\P{x}".chars().map(Token::Char).collect::<Vec<_>>()
        );
    }

    #[test]
    fn mtext_codes() {
        use Token::*;
        assert_eq!(
            all(r"{\H2.5x;A}\P\~\\\{", TextKind::MText),
            [
                GroupStart,
                Property {
                    code: 'H',
                    value: "2.5x"
                },
                Char('A'),
                GroupEnd,
                Break(super::Break::Paragraph),
                NonBreakingSpace,
                Char('\\'),
                Char('{'),
            ]
        );
        assert_eq!(
            all(r"3\S1#2;", TextKind::MText)[1],
            Stack {
                top: "1",
                bottom: "2",
                separator: '#'
            }
        );
        assert_eq!(all(r"\Sab;", TextKind::MText)[0], StackUnsplit("ab"));
        assert_eq!(
            all(r"\L\l\K", TextKind::MText)[..2],
            [
                Toggle(super::Toggle::Underline(Switch::On)),
                Toggle(super::Toggle::Underline(Switch::Off))
            ]
        );
    }

    #[test]
    fn what_is_not_a_closed_code_is_kept_as_text() {
        use Token::*;
        assert_eq!(all(r"\A1", TextKind::MText)[0], Unknown(r"\A"));
        assert_eq!(all(r"\Z", TextKind::MText)[0], Unknown(r"\Z"));
        assert_eq!(all("\\", TextKind::MText), [Unknown("\\")]);
        // A storage escape a reader left (an ASCII one) is not a code.
        assert_eq!(all(r"\U+005C", TextKind::MText)[0], Unknown(r"\U"));
    }

    #[test]
    fn nothing_is_lost_and_nothing_is_read_twice() {
        // What a code produces is not read again: `\\P` is a backslash and
        // a P, not a paragraph break.
        use Token::*;
        assert_eq!(all(r"\\P", TextKind::MText), [Char('\\'), Char('P')]);
        assert_eq!(all("%%%d", TextKind::Line), [Char('%'), Char('d')]);
    }
}
