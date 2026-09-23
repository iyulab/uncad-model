//! How a drawing's strings are stored, undone.
//!
//! A file whose text is in a code page cannot hold every character, so the
//! formats write the ones it cannot hold as escapes, in any string -- a
//! text's value, a layer's name, a dictionary key:
//!
//! - `\U+XXXX` is the Unicode character with that code point (four hex
//!   digits);
//! - `\M+nXXXX` is a character of an Asian code page (`n` from 1 to 5 names
//!   it, `XXXX` is its two bytes);
//! - in a DXF file, whose every value is one line, a control character is
//!   written in caret notation: `^J` is a line feed, `^I` a tab, and `^ ` (a
//!   space after the caret) is the caret itself.
//!
//! This is storage, not text: the file means the character, and two files
//! that store the same string differently mean the same string. A reader
//! undoes it as it takes the string out of the file (principles §6.1), so the
//! model's strings do not depend on which way a file stored them. What the
//! text itself says in its control codes -- `%%d`, MTEXT's `\P` -- is not
//! storage, and is left alone here.
//!
//! An escape that names a character every code page holds (anything below
//! U+0080) is left as written. No writer needs one, and undoing it could make
//! a character the text reads as a code: `\U+005C` would become the
//! backslash an MTEXT code starts with.

use std::borrow::Cow;

/// `text` with its `\U+XXXX` and `\M+nXXXX` escapes replaced by the
/// characters they store.
///
/// `mbcs` decodes a `\M+` escape: it is given the Windows code page the
/// escape names (932, 950, 949, 1361 or 936) and the character's two bytes,
/// and returns the character, or `None` when it cannot -- the escape is then
/// left as written. A doubled backslash is kept as it is and never starts
/// an escape.
pub fn decode_escapes(text: &str, mbcs: impl Fn(u16, [u8; 2]) -> Option<char>) -> Cow<'_, str> {
    if !text.contains('\\') {
        return Cow::Borrowed(text);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            Some('\\') => {
                out.push_str("\\\\");
                i += 2;
                continue;
            }
            Some('U') => {
                if let Some(c) = unicode(&chars[i + 2..]) {
                    out.push(c);
                    i += 7;
                    continue;
                }
            }
            Some('M') => {
                if let Some(c) = multibyte(&chars[i + 2..], &mbcs) {
                    out.push(c);
                    i += 8;
                    continue;
                }
            }
            _ => {}
        }
        out.push('\\');
        i += 1;
    }
    Cow::Owned(out)
}

/// `text` with its caret notation undone: `^` followed by `@`, `A` to `Z`,
/// `[`, `\`, `]`, `^` or `_` is the control character that letter names
/// (`^J` a line feed), and `^ ` is the caret. Any other caret is left as
/// written. DXF only -- a binary format stores control characters as they
/// are.
pub fn decode_caret(text: &str) -> Cow<'_, str> {
    if !text.contains('^') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '^' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some(' ') => {
                out.push('^');
                chars.next();
            }
            Some(next @ '@'..='_') => {
                out.push(char::from(next as u8 - 0x40));
                chars.next();
            }
            _ => out.push('^'),
        }
    }
    Cow::Owned(out)
}

/// `+XXXX` after `\U`: the character, when it is one no code page lacks.
fn unicode(rest: &[char]) -> Option<char> {
    if rest.first() != Some(&'+') {
        return None;
    }
    let code = hex(rest.get(1..5)?)?;
    char::from_u32(code).filter(|c| !c.is_ascii())
}

/// `+nXXXX` after `\M`: `n` names the code page, `XXXX` the two bytes.
fn multibyte(rest: &[char], mbcs: &impl Fn(u16, [u8; 2]) -> Option<char>) -> Option<char> {
    if rest.first() != Some(&'+') {
        return None;
    }
    let codepage = match rest.get(1)? {
        '1' => 932,
        '2' => 950,
        '3' => 949,
        '4' => 1361,
        '5' => 936,
        _ => return None,
    };
    let bytes = hex(rest.get(2..6)?)?;
    let c = mbcs(codepage, [(bytes >> 8) as u8, bytes as u8])?;
    (!c.is_ascii()).then_some(c)
}

fn hex(digits: &[char]) -> Option<u32> {
    if digits.len() != 4 || !digits.iter().all(char::is_ascii_hexdigit) {
        return None;
    }
    u32::from_str_radix(&digits.iter().collect::<String>(), 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: u16, _: [u8; 2]) -> Option<char> {
        None
    }

    #[test]
    fn a_unicode_escape_is_the_character_it_stores() {
        assert_eq!(decode_escapes(r"108\U+00B0", none), "108\u{b0}");
        assert_eq!(decode_escapes(r"\U+0410\U+043D", none), "\u{410}\u{43d}");
        // Lower-case hex digits too.
        assert_eq!(decode_escapes(r"\U+00b1", none), "\u{b1}");
    }

    #[test]
    fn what_is_not_an_escape_is_left_as_written() {
        for s in [
            r"\P\U+00",    // too few digits
            r"\U00B0",     // no plus
            r"\U+00G0",    // not hex
            r"\U+005C",    // a character every code page holds
            r"\U+0041",    // the same
            r"\A1;x\P",    // MTEXT codes are text, not storage
            r"%%c32",      // so are percent codes
            r"C:\\U+00B0", // a doubled backslash starts nothing
        ] {
            assert_eq!(decode_escapes(s, none), s, "{s}");
        }
    }

    #[test]
    fn a_multibyte_escape_goes_through_the_code_page_it_names() {
        let korean = |cp: u16, b: [u8; 2]| (cp == 949 && b == [0xB5, 0xB5]).then_some('\u{b3c4}');
        assert_eq!(decode_escapes(r"\M+3B5B5", korean), "\u{b3c4}");
        // A code page the caller cannot decode leaves the escape.
        assert_eq!(decode_escapes(r"\M+1B5B5", korean), r"\M+1B5B5");
        assert_eq!(decode_escapes(r"\M+9B5B5", korean), r"\M+9B5B5");
    }

    #[test]
    fn caret_notation_is_the_control_character_it_names() {
        assert_eq!(decode_caret("10^J20"), "10\n20");
        assert_eq!(decode_caret("a^Ib"), "a\tb");
        assert_eq!(decode_caret("x^ 2"), "x^2");
        // Anything else after a caret is left as written.
        assert_eq!(decode_caret("x^2"), "x^2");
        assert_eq!(decode_caret("end^"), "end^");
        assert_eq!(decode_caret("plain"), "plain");
    }
}
