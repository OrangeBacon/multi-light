use std::{
    path::{Path, PathBuf},
    str::CharIndices,
    sync::LazyLock,
};

use onig::{Captures, Regex};

use crate::Error;

use super::{Config, ConfigTree};

impl Config {
    /// Parse a plist string
    // (Note that the vscode version of this parser is pretty different)
    pub fn from_plist(
        file_name: impl Into<PathBuf>,
        content: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let tree = PlistParser::new(content.as_ref(), &file_name);

        Ok(Self {
            tree: tree.parse()?,
            file_name,
        })
    }
}

/// Simple plist parser.  Plist is roughly XML files with much of the complexity
/// of XML skipped.
struct PlistParser<'a> {
    chars: CharIndices<'a>,
    file_name: &'a Path,
}

/// A single XML tag.  if is_closed, the tag counts as self closing
struct Tag<'a> {
    name: &'a str,
    is_closed: bool,
}

/// Events that can be emitted by a file format parser
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParserEvent {
    Value(ConfigTree),
    EnterDict,
    Key(String),
    CloseDict,
    EnterArray,
    CloseArray,
    Eof,
}

impl<'a> PlistParser<'a> {
    /// Create a new parser
    fn new(input: &'a str, file_name: &'a Path) -> Self {
        Self {
            chars: input.char_indices(),
            file_name,
        }
    }

    /// Run the parser over the input code
    fn parse(mut self) -> Result<ConfigTree, Error> {
        if self.peek() == Some('\u{65279}') {
            self.chars.next();
        }

        loop {
            let ev = self.parse_value()?;
            if ev == ParserEvent::Eof {
                break;
            }
            println!("{ev:?}");
        }

        todo!()
    }

    /// Accept any single value from the input
    fn parse_value(&mut self) -> Result<ParserEvent, Error> {
        loop {
            self.skip_whitespace();

            let Some((_, next)) = self.chars.next() else {
                return Ok(ParserEvent::Eof);
            };

            if next != '<' {
                return Err(self.error("expected <"));
            }

            let Some(peek) = self.peek() else {
                return Err(self.error("unexpected end of input"));
            };

            match peek {
                // skip parts of the syntax that can be treated like comments,
                // once the comment-like is matched then it goes back to the start
                // of the loop and tries to match a new piece of syntax.
                '?' => {
                    self.chars.next();
                    self.advance_until("?>");
                }
                '!' => {
                    self.chars.next();
                    if self.chars.as_str().starts_with("--") {
                        self.advance_until("-->");
                    } else {
                        self.advance_until(">");
                    }
                }
                '/' => {
                    self.chars.next();
                    self.skip_whitespace();
                    if self.chars.as_str().starts_with("plist") {
                        self.advance_until(">");
                    } else if self.chars.as_str().starts_with("dict") {
                        self.advance_until(">");
                        return Ok(ParserEvent::CloseDict);
                    } else if self.chars.as_str().starts_with("array") {
                        self.advance_until(">");
                        return Ok(ParserEvent::CloseArray);
                    } else {
                        return Err(self.error("unexpected closed tag"));
                    }
                }
                // parse actual tags
                _ => return self.parse_tag(),
            }
        }
    }

    /// Parse the content of an actual value-containing XML tag
    fn parse_tag(&mut self) -> Result<ParserEvent, Error> {
        let tag = self.parse_open_tag();

        match tag.name {
            "dict" => Ok(ParserEvent::EnterDict),
            "array" => Ok(ParserEvent::EnterArray),
            "key" => Ok(ParserEvent::Key(self.parse_tag_value(tag)?)),

            "string" | "real" | "integer" | "date" | "data" => Ok(ParserEvent::Value(
                ConfigTree::String(self.parse_tag_value(tag)?),
            )),
            "true" => Ok(ParserEvent::Value(ConfigTree::Bool(true))),
            "false" => Ok(ParserEvent::Value(ConfigTree::Bool(false))),
            _ if tag.name.starts_with("plist") => self.parse_value(),
            tag => {
                let tag = tag.to_string();
                Err(self.error(format!("unexpected opened tag {tag}")))
            }
        }
    }

    /// Parse an XML tag starting directly after the opening `<`.
    fn parse_open_tag(&mut self) -> Tag<'a> {
        let mut res = self.capture_until(">");

        let mut is_closed = false;
        if res.ends_with('/') {
            res = &res[0..res.len() - 1];
            is_closed = true;
        }

        Tag {
            name: res,
            is_closed,
        }
    }

    /// Parse the text value within a single XML tag
    fn parse_tag_value(&mut self, tag: Tag<'a>) -> Result<String, Error> {
        if tag.is_closed {
            return Ok(String::new());
        }

        let val = self.capture_until("</");
        self.advance_until(">");

        self.escape_value(val)
    }

    fn escape_value(&self, input: &str) -> Result<String, Error> {
        static DECIMAL: LazyLock<Regex> = LazyLock::new(|| Regex::new("&#([0-9]+);").unwrap());
        static HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("&#x([0-9a-f]+);").unwrap());
        static OTHER: LazyLock<Regex> =
            LazyLock::new(|| Regex::new("&amp;|&lt;|&gt;|&quot;|&apos;").unwrap());

        let input = replace_all(&DECIMAL, input, |cap: &Captures<'_>| {
            let cap = cap.at(1).unwrap_or("");
            let num = cap
                .parse::<u32>()
                .map_err(|_| self.error("unable to parse decimal number"))?;
            Ok(String::from(
                char::from_u32(num).ok_or_else(|| self.error("invalid decimal escape"))?,
            ))
        })?;

        let input = replace_all(&HEX, &input, |cap: &Captures<'_>| {
            let cap = cap.at(1).unwrap_or("");
            let num = u32::from_str_radix(cap, 16)
                .map_err(|_| self.error("unable to parse hex number"))?;
            Ok(String::from(
                char::from_u32(num).ok_or_else(|| self.error("invalid hex escape"))?,
            ))
        })?;

        replace_all(&OTHER, &input, |cap: &Captures<'_>| {
            let item = match cap.at(1).unwrap_or("") {
                "&amp;" => "&",
                "&lt;" => "<",
                "&gt;" => ">",
                "&quot;" => "\"",
                "&apos;" => "\\",
                _ => return Err(self.error("internal regex error")),
            };

            Ok(String::from(item))
        })
    }

    /// Skip any whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(peek) = self.peek() {
            if !" \t\r\n".contains(peek) {
                break;
            }

            self.chars.next();
        }
    }

    /// advance until the next occurrence of the provided string, including the
    /// provided input.
    fn advance_until(&mut self, search: &str) {
        loop {
            if self.chars.as_str().starts_with(search) {
                break;
            }

            if self.chars.next().is_none() {
                return;
            }
        }

        self.chars.nth(search.len() - 1);
    }

    /// Same as advance_until, but returns the text content that was advanced over
    fn capture_until(&mut self, search: &str) -> &'a str {
        let res = self.chars.as_str();
        let start_offset = self.chars.offset();
        self.advance_until(search);
        let end_offset = self.chars.offset() - search.len();

        &res[0..end_offset - start_offset]
    }

    /// Return the next character in the stream
    fn peek(&self) -> Option<char> {
        self.chars.clone().next().map(|(_, c)| c)
    }

    /// Create an error message
    fn error(&self, err: impl Into<String>) -> Error {
        let sample: String = self.chars.clone().take(50).map(|(_, c)| c).collect();

        let err = if sample.is_empty() {
            format!("at file end: {} ", err.into())
        } else {
            format!(
                "near offset {}: {} ~~~ {} ~~~",
                self.chars.offset(),
                err.into(),
                sample
            )
        };

        Error::PlistError {
            err,
            file_name: self.file_name.to_path_buf(),
        }
    }
}

/// Wrapper for regex replace that allows a replacement to fail
fn replace_all<E>(
    re: &Regex,
    input: &str,
    replace: impl Fn(&Captures) -> Result<String, E>,
) -> Result<String, E> {
    let mut new = String::with_capacity(input.len());
    let mut last_match = 0;

    for caps in re.captures_iter(input) {
        let Some((start, end)) = caps.pos(0) else {
            continue;
        };

        new.push_str(&input[last_match..start]);
        new.push_str(&replace(&caps)?);
        last_match = end;
    }

    new.push_str(&input[last_match..]);
    Ok(new)
}
