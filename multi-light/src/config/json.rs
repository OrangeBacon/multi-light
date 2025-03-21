use std::{borrow::Cow, path::PathBuf, str::CharIndices, sync::LazyLock};

use onig::{Captures, Regex};

use crate::Error;

use super::{Config, ConfigNodeID, ConfigTree, Location};

impl Config {
    /// Parse a JSON string
    pub fn from_json(
        file_name: impl Into<PathBuf>,
        content: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let json = serde_json::from_str(content.as_ref()).map_err(|err| Error::SerdeJson {
            err,
            file_name: file_name.clone(),
        })?;

        Ok(Self::NonDebug {
            tree: json,
            file_name,
        })
    }

    /// Parse a JSON string, including debug information.  See
    /// https://github.com/microsoft/vscode-textmate/blob/v9.2.0/src/json.ts for
    /// the parsing algorithm used (i.e. this one should match what that one outputs)
    pub fn from_json_debug(
        file_name: impl Into<PathBuf>,
        source: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let mut parser = JSONParser::new(file_name, source.as_ref());

        parser.parse()
    }
}

/// State used while parsing a single JSON file
struct JSONParser<'a> {
    file_name: PathBuf,

    /// source code to be parsed
    chars: CharIndices<'a>,

    /// current line number
    line: usize,

    /// current column number
    column: usize,

    /// stack of partially parsed trees
    object_stack: Vec<ConfigTree<ConfigNodeID>>,

    /// stack of which state the parser is in
    state_stack: Vec<JSONState>,

    /// The currently in use tree
    current_object: Option<ConfigTree<ConfigNodeID>>,

    /// The currently in use state
    current_state: JSONState,

    /// List of all stored locations
    location: Vec<Location>,
}

/// All token types within a JSON file
#[derive(Debug)]
enum JSONTokenKind {
    String,
    LeftSquare,
    LeftCurly,
    RightSquare,
    RightCurly,
    Colon,
    Comma,
    Null,
    True,
    False,
    Number,
}

/// Current state of the JSON parser
#[derive(Debug, Clone, Copy)]
enum JSONState {
    Root,
    Dict,
    DictComma,
    DictNoClose,
    Array,
    ArrayComma,
    ArrayNoClose,
}

/// A single token parsed from a JSON file
#[derive(Debug)]
struct JSONToken<'a> {
    /// String value of the token, string tokens will have any escapes processed
    /// and the result stored here.
    value: Option<Cow<'a, str>>,

    /// What sort of token this is
    kind: JSONTokenKind,

    /// offset into the source string in bytes for the start of this token
    byte_offset: usize,

    /// line number of the start of the token
    line: usize,

    /// column number of the start of the token
    column: usize,
}

impl<'a> JSONParser<'a> {
    fn new(file_name: PathBuf, source: &'a str) -> Self {
        JSONParser {
            file_name,
            chars: source.char_indices(),
            line: 1,
            column: 1,
            object_stack: vec![],
            state_stack: vec![],
            current_object: None,
            current_state: JSONState::Root,
            location: vec![],
        }
    }

    /// Get the next full token from the source
    fn next_token(&mut self) -> Result<Option<JSONToken<'a>>, Error> {
        // skip whitespace
        let mut current = None;
        let mut current_str = self.chars.as_str();

        while let Some(ch) = self.advance() {
            current = Some(ch);
            if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
                current_str = self.chars.as_str();
                continue;
            } else {
                break;
            }
        }

        let Some(current) = current else {
            return Ok(None);
        };
        let token_loc = self.location();

        let single = |str: &'static str, kind| {
            Ok(Some(JSONToken {
                value: Some(Cow::Borrowed(str)),
                kind,
                byte_offset: token_loc.byte_number,
                line: token_loc.line_number,
                column: token_loc.column_number,
            }))
        };

        let mut multi = |str: &'static str, kind| {
            // check all chars other than the first because the first was already
            // checked before calling this function
            for ch in str.chars().skip(1) {
                if self.advance() != Some(ch) {
                    return Ok(None);
                }
            }
            Ok(Some(JSONToken {
                value: Some(Cow::Borrowed(str)),
                kind,
                byte_offset: token_loc.byte_number,
                line: token_loc.line_number,
                column: token_loc.column_number,
            }))
        };

        match current {
            '[' => single("[", JSONTokenKind::LeftSquare),
            '{' => single("{", JSONTokenKind::LeftCurly),
            ']' => single("]", JSONTokenKind::RightSquare),
            '}' => single("}", JSONTokenKind::RightCurly),
            ':' => single(":", JSONTokenKind::Colon),
            ',' => single(",", JSONTokenKind::Comma),
            'n' => multi("null", JSONTokenKind::Null),
            't' => multi("true", JSONTokenKind::True),
            'f' => multi("false", JSONTokenKind::False),
            '"' => {
                let str = self.chars.as_str();
                let start_offset = self.chars.offset();
                let mut eof = true;
                while let Some(ch) = self.advance() {
                    if ch == '\\' {
                        // skip next
                        self.advance();
                        continue;
                    }
                    if ch == '"' {
                        eof = false;
                        break;
                    }
                }

                if eof {
                    // the loop exited due to advance running out of characters
                    // and not due to a double quote
                    return Ok(None);
                }

                // get the string content
                let value = &str[..(self.chars.offset() - start_offset - 1)];
                let value = self.json_string_value(value)?;

                Ok(Some(JSONToken {
                    value: Some(Cow::Owned(value.to_string())),
                    kind: JSONTokenKind::String,
                    byte_offset: token_loc.byte_number,
                    line: token_loc.line_number,
                    column: token_loc.column_number,
                }))
            }
            _ => {
                // must be a number as none of the other tokens match
                let start_offset = self.chars.offset();

                while self
                    .advance_if(|ch| ".1234567890eE+-".contains(ch))
                    .is_some()
                {}

                let value = &current_str[..(self.chars.offset() - start_offset)];

                Ok(Some(JSONToken {
                    value: Some(Cow::Owned(value.to_string())),
                    kind: JSONTokenKind::Number,
                    byte_offset: token_loc.byte_number,
                    line: token_loc.line_number,
                    column: token_loc.column_number,
                }))
            }
        }
    }

    /// get the next character and update line/column positions
    fn advance(&mut self) -> Option<char> {
        let (_, ch) = self.chars.next()?;

        if ch == '\n' {
            self.column = 0;
            self.line += 1;
        } else {
            self.column += 1;
        }

        Some(ch)
    }

    /// get the next character but only consume it if the function returns true.
    fn advance_if(&mut self, f: impl Fn(char) -> bool) -> Option<char> {
        let mut iter = self.chars.clone();
        let (_, next) = iter.next()?;
        if f(next) { self.advance() } else { None }
    }

    /// Get the current location
    fn location(&self) -> Location {
        Location {
            line_number: self.line,
            column_number: self.column,
            byte_number: self.chars.offset(),
        }
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

        Error::JSONError {
            err,
            file_name: self.file_name.clone(),
        }
    }

    /// process escape sequences within a JSON string
    fn json_string_value<'b>(&self, input: &'b str) -> Result<Cow<'b, str>, Error> {
        static UNICODE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\\u([0-9A-Fa-f]{4})").unwrap());
        static OTHER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\(.)").unwrap());

        if !input.contains('\\') {
            return Ok(Cow::Borrowed(input));
        }

        let out = replace_all(&UNICODE, input, |cap: &Captures| {
            let num =
                u32::from_str_radix(cap.at(1).unwrap_or(""), 16).map_err(|e| Error::JSONError {
                    err: format!("{} while parsing unicode escape", e),
                    file_name: self.file_name.clone(),
                })?;

            let ch = char::from_u32(num).unwrap();

            Ok(String::from(ch))
        })?;

        let out = replace_all(&OTHER, &out, |cap: &Captures| {
            let ch = match cap.at(1).unwrap_or("") {
                "\"" => '"',
                "\\" => '\\',
                "/" => '/',
                "b" => '\u{8}',
                "f" => '\u{C}',
                "n" => '\n',
                "r" => '\r',
                "t" => '\t',
                _ => {
                    return Err(Error::JSONError {
                        err: "invalid escape sequence".into(),
                        file_name: self.file_name.clone(),
                    });
                }
            };

            Ok(String::from(ch))
        })?;

        Ok(Cow::Owned(out))
    }

    /// Parse the input into a tree.  Only ever outputs a tree with debug info as
    /// serde_json can likely do better for the without option.
    fn parse(&mut self) -> Result<Config, Error> {
        while let Some(tok) = self.next_token()? {
            println!("{tok:?}");
        }

        todo!()
    }
}

impl JSONToken<'_> {
    fn location(&self) -> Location {
        Location {
            line_number: self.line,
            column_number: self.column,
            byte_number: self.byte_offset,
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
