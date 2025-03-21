# Multi-Light
Rust syntax highlighting library

This library is designed to provide a common interface for the use of
`tmLanguage`, `sublime-syntax` and tree-sitter grammars.

The following theme files are all supported:
`tmTheme`, `sublime-theme`, `color-theme.json` (Visual Studio Code theme format)
and tree-sitter theme files.

Any theme file format can be combined with any syntax format.  If a language
uses another one as an injection (e.g. showing JavaScript within an HTML file)
the syntaxes can use different descriptions, there is no requirement for an
injection syntax to be the same as the one requesting its use.

This library does not come with any built-in syntaxes or themes, they should all
be provided by the user of the library.

Parts of the implementation of this library are ported from Visual Studio Code's
[vs-code texmate repository](https://github.com/microsoft/vscode-textmate/tree/main).
See `LICENSE` for more information.
