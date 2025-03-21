use std::{borrow::Cow, collections::HashMap, path::PathBuf, str::CharIndices, sync::LazyLock};

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

        let parser = JSONParser::new(file_name, source.as_ref());

        parser.parse()
    }
}

type Tree = ConfigTree<ConfigNodeID>;

/// State used while parsing a single JSON file
struct JSONParser<'a> {
    file_name: PathBuf,

    /// source code to be parsed
    chars: CharIndices<'a>,

    /// current line number
    line: usize,

    /// current column number
    column: usize,

    /// stack of which state the parser is in
    stack: Vec<JSONState>,

    /// List of all stored locations
    location: Vec<Location>,
}

/// All token types within a JSON file
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug, Clone)]
enum JSONState {
    /// Within a dictionary / map
    Dict {
        id: ConfigNodeID,
        value: HashMap<String, Tree>,
        key: String,
    },

    /// within an array
    Array { id: ConfigNodeID, value: Vec<Tree> },
}

/// A single token parsed from a JSON file
#[derive(Debug)]
struct JSONToken {
    /// String value of the token, string tokens will have any escapes processed
    /// and the result stored here.
    value: String,

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
            stack: vec![],
            location: vec![],
        }
    }

    /// Get the next full token from the source
    fn next_token(&mut self) -> Result<Option<JSONToken>, Error> {
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
        let token_loc = self.current_location();

        let single = |str: &'static str, kind| {
            Ok(Some(JSONToken {
                value: String::from(str),
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
                value: String::from(str),
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
                    value: value.to_string(),
                    kind: JSONTokenKind::String,
                    byte_offset: token_loc.byte_number,
                    line: token_loc.line_number,
                    column: token_loc.column_number,
                }))
            }
            start => {
                // must be a number as none of the other tokens match
                let start_offset = self.chars.offset() - start.len_utf8();

                while self
                    .advance_if(|ch| ".1234567890eE+-".contains(ch))
                    .is_some()
                {}

                let value = &current_str[..(self.chars.offset() - start_offset)];

                Ok(Some(JSONToken {
                    value: value.to_string(),
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
    fn current_location(&self) -> Location {
        Location {
            line_number: self.line,
            column_number: self.column,
            byte_number: self.chars.offset(),
        }
    }

    /// Get a node ID for the given location
    fn node_id(&mut self, location: Location) -> ConfigNodeID {
        let id = self.location.len();

        self.location.push(location);

        ConfigNodeID(id)
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
    fn parse(mut self) -> Result<Config, Error> {
        // This parser is written differently from that used in vscode, however
        // hopefully not differently on its output.  Vscode uses significantly
        // more states and uses the following loop to read tokens from the inside
        // of an array or dictionary, whereas this parser puts more focus onto the
        // code running within each state.  The reason for this change is that
        // vscode can push a value into an array or store it in a dictionary and
        // then still mutate it in a different method and i cannot see a way to
        // copy their exact method without making everything in the tree
        // Rc<Refcell> or Arc<Mutex> which seems rather complicated compared with
        // just re-organising the code while porting it from javascript to rust.
        // would it have been better to completely ignore the original parser and
        // just write recursive-descent?  could have been easier at least but i guess
        // this works better if you have thousands of nested items?
        while let Some(tok) = self.next_token()? {
            match self.stack.pop() {
                None => self.parse_root(tok)?,
                Some(JSONState::Dict { id, value, .. }) => self.parse_dict(tok, id, value)?,
                Some(JSONState::Array { id, value }) => self.parse_array(tok, id, value)?,
            }
        }

        if self.stack.len() != 1 {
            return Err(self.error("unclosed constructs"));
        }

        // will always succeed due to check above
        let tree = self.stack.pop().map(JSONState::into_tree).unwrap();

        Ok(Config::Debug {
            tree,
            location: self.location,
            file_name: self.file_name,
        })
    }

    /// JSON Root state parsing (nothing else has been parsed yet)
    fn parse_root(&mut self, tok: JSONToken) -> Result<(), Error> {
        if !self.stack.is_empty() {
            return Err(self.error("too many constructs in root"));
        }

        if tok.kind == JSONTokenKind::LeftCurly {
            let id = self.node_id(tok.location());
            self.stack.push(JSONState::Dict {
                id,
                value: HashMap::new(),
                key: String::new(),
            });

            Ok(())
        } else if tok.kind == JSONTokenKind::LeftSquare {
            let id = self.node_id(tok.location());
            self.stack.push(JSONState::Array { id, value: vec![] });

            Ok(())
        } else {
            Err(self.error("unexpected token in root"))
        }
    }

    /// Parse a dictionary / map
    fn parse_dict(
        &mut self,
        mut tok: JSONToken,
        id: ConfigNodeID,
        mut dict: HashMap<String, Tree>,
    ) -> Result<(), Error> {
        if tok.kind == JSONTokenKind::RightCurly {
            // the dictionary is finished, so put the constructed value into the
            // tree segment lower in the stack
            self.consume_state(JSONState::Dict {
                id,
                value: dict,
                key: String::new(),
            });

            return Ok(());
        }

        // we are not at the end of the dict, so parse the next value
        if !dict.is_empty() {
            if tok.kind != JSONTokenKind::Comma {
                // we had a previous value, so we need a comma before the next one
                return Err(self.error("expected , or }"));
            }

            // get the token after the comma which should be the key of the dict
            match self.next_token()? {
                Some(t) => tok = t,
                None => return Ok(()),
            };
        }

        if tok.kind != JSONTokenKind::String {
            return Err(self.error("unexpected token in dict"));
        }

        let key = tok.value;

        match self.next_token()? {
            Some(JSONToken {
                kind: JSONTokenKind::Colon,
                ..
            }) => {}
            Some(_) => return Err(self.error("expected colon")),
            None => return Ok(()),
        };

        let Some(tok) = self.next_token()? else {
            return Err(self.error("expected value"));
        };

        let value = match tok.kind {
            JSONTokenKind::String
            | JSONTokenKind::Null
            | JSONTokenKind::True
            | JSONTokenKind::False
            | JSONTokenKind::Number => self.token_to_tree(tok),
            JSONTokenKind::LeftSquare => {
                // push the current dict so we can get back into it.
                self.stack.push(JSONState::Dict {
                    id,
                    value: dict,
                    key,
                });
                let array_id = self.node_id(tok.location());

                // push the array so we can enter the array parser
                self.stack.push(JSONState::Array {
                    id: array_id,
                    value: vec![],
                });

                return Ok(());
            }
            JSONTokenKind::LeftCurly => {
                // push the current dict so we can get back into it.
                self.stack.push(JSONState::Dict {
                    id,
                    value: dict,
                    key,
                });
                let array_id = self.node_id(tok.location());

                // push the dict so we can enter the dict parser
                self.stack.push(JSONState::Dict {
                    id: array_id,
                    value: HashMap::new(),
                    key: String::new(),
                });

                return Ok(());
            }
            _ => return Err(self.error("unexpected token in dict")),
        };

        dict.insert(key, value);

        // continue with the dict in the next iteration of the main parse loop
        self.stack.push(JSONState::Dict {
            id,
            value: dict,
            key: String::new(),
        });
        Ok(())
    }

    /// Parse a dictionary / map
    fn parse_array(
        &mut self,
        mut tok: JSONToken,
        id: ConfigNodeID,
        mut arr: Vec<Tree>,
    ) -> Result<(), Error> {
        if tok.kind == JSONTokenKind::RightSquare {
            // the dictionary is finished, so put the constructed value into the
            // tree segment lower in the stack
            self.consume_state(JSONState::Array { id, value: arr });

            return Ok(());
        }

        // we are not at the end of the dict, so parse the next value
        if !arr.is_empty() {
            if tok.kind != JSONTokenKind::Comma {
                // we had a previous value, so we need a comma before the next one
                return Err(self.error("expected , or ]"));
            }

            // get the token after the comma which should be the key of the dict
            match self.next_token()? {
                Some(t) => tok = t,
                None => return Ok(()),
            };
        }

        let value = match tok.kind {
            JSONTokenKind::String
            | JSONTokenKind::Null
            | JSONTokenKind::True
            | JSONTokenKind::False
            | JSONTokenKind::Number => self.token_to_tree(tok),
            JSONTokenKind::LeftSquare => {
                // push the current array so we can get back into it.
                self.stack.push(JSONState::Array { id, value: arr });
                let array_id = self.node_id(tok.location());

                // push the array so we can enter the array parser
                self.stack.push(JSONState::Array {
                    id: array_id,
                    value: vec![],
                });

                return Ok(());
            }
            JSONTokenKind::LeftCurly => {
                // push the current array so we can get back into it.
                self.stack.push(JSONState::Array { id, value: arr });
                let array_id = self.node_id(tok.location());

                // push the dict so we can enter the dict parser
                self.stack.push(JSONState::Dict {
                    id: array_id,
                    value: HashMap::new(),
                    key: String::new(),
                });

                return Ok(());
            }
            _ => return Err(self.error("unexpected token in dict")),
        };

        arr.push(value);

        // continue with the array in the next iteration of the main parse loop
        self.stack.push(JSONState::Array { id, value: arr });
        Ok(())
    }

    /// take a completed array/dict and put it into the right place in the tree
    fn consume_state(&mut self, state: JSONState) {
        match self.stack.last_mut() {
            Some(JSONState::Array { value, .. }) => value.push(state.into_tree()),
            Some(JSONState::Dict { value, key, .. }) => {
                value.insert(std::mem::take(key), state.into_tree());
            }
            None => self.stack.push(state),
        }
    }

    /// convert a token into its config tree value
    fn token_to_tree(&mut self, tok: JSONToken) -> Tree {
        let id = self.node_id(tok.location());

        match tok.kind {
            JSONTokenKind::String => ConfigTree::String {
                id,
                value: tok.value,
            },
            JSONTokenKind::True => ConfigTree::Bool { id, value: true },
            JSONTokenKind::False => ConfigTree::Bool { id, value: false },
            JSONTokenKind::Number => ConfigTree::String {
                id,
                value: tok.value,
            },

            // either null or a default value for something that cannot be converted
            // into a tree as this is only called within other match statements
            // that already check for this
            _ => ConfigTree::Null { id },
        }
    }
}

impl JSONToken {
    fn location(&self) -> Location {
        Location {
            line_number: self.line,
            column_number: self.column,
            byte_number: self.byte_offset,
        }
    }
}

impl JSONState {
    /// Convert this state into a tree node
    fn into_tree(self) -> Tree {
        match self {
            JSONState::Dict { id, value, .. } => ConfigTree::Object { id, value },
            JSONState::Array { id, value } => ConfigTree::Array { id, value },
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
