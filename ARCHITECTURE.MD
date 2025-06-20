References
- VSCode theme files:
  https://code.visualstudio.com/api/extension-guides/color-theme
  https://arnav.tech/creating-and-publishing-visual-studio-code-color-themes
- tmTheme files:
  https://www.sublimetext.com/docs/color_schemes_tmtheme.html
- sublime themes:
  https://www.sublimetext.com/docs/color_schemes.html
- sublime's updated replacement for tmLanguage:
  https://www.sublimetext.com/docs/syntax.html
- Tree-sitter grammars + highlighting
- theme colour space info:
  https://hardtoc.com/2014/09/03/not-quite-monokai.html
- tmLanguage implementation used by vscode:
  https://github.com/microsoft/vscode-textmate

File Format Parsers (cargo features!)
- PList XML
- TOML
- JSON
- YAML

File Data (cargo features!)
- tmTheme
- tmLanguage
- sublime-theme
- sublime-syntax

Input
- create a trait that allows for:
  - string which could contain one of the file data types in one of the file formats
  - struct containing a string and a positive identification of its file data and/or file format
  - struct containing tree-sitter highlighting data (cargo feature)
  - all options would require a name for the input
  - duplicate name = merge with other file of the same type, with the new one overriding the old one.
- struct Syntax + struct Theme with their own constructor, methods, names, etc
- source text = &str
- no provided grammars, instead the configuration should provide a method of loading grammars based on their name
- registry to store parsed grammars and themes
- all input files are obtained by the user of this library i.e. I don't do file system access to find anything
- make the internal format of a grammar allow for all data from any of the input grammar kinds (+ same for themes)

struct Registry {
  fn new() -> Registry; // new registry
  fn read_file(fn(&str)->Result<impl HighlightInput>); // function for reading a required dependency file
  fn add(name: &str, impl HighlightInput) -> Result<()>; // add a file to the registry
  // Get the theme for a given name (or default if there isn't one already).  Allows for more complex construction of themes, i.e.
  // if you want to merge them, read them, modify them based on code, etc. (do the same for grammars)
  fn theme(name: &str) -> Theme<'a>;
  fn syntax(name: &str) -> Syntax<'a>;

  // parse the document.  If using the mut parser, allow calling the read_file callback, otherwise the registry cannot
  // be changed, so return an error if a file is not found.
  // allow for language detection based on the available grammars, or, provide a language name
  fn parse(&self, &str) -> output
  fn parse_mut(&mut self, &str) -> output
}

Output
- parse function - output iterator of structs:
  - source - reference to the input source text (make this optional, i.e. add an additional parse function with no lifetime parameters)
  - offset - byte offset into the source text
  - length - byte length of source text that is included in this span
  - style - text styling rules
    - foreground colour
    - background colour (optional?)
    - underline, italic, bold, ...
  - somehow need to output a background colour for the whole input code block, not just the bits where text is present
    - or maybe per line? (how do diff styles work?)
- colours
  - theme colours could be in generic RGB (default if unspecified) or sRGB, convert all into sRGB for output
  - colours provided as 4no. rgba u8 values.
  - no support for other colour spaces outside RGB and sRGB
  - need to implement Sublime Text colour functions (ugh, thats a lot for colour)
  - implement the colour blending/different colours depending on hash of the token, etc
- text editor specific styles should be readable from a parsed theme but not used in the parsed output.
struct ParseOutput {
  whole code block style options
  iterator over each line of code {
    Option<individual line style options>
    iterator over each span within the line {
      data about each highlighted span (see above)
    }
  }
}

Questions
- how to combine tree-sitter grammars with the other kinds
  - if tmLanguage has an injection that is implemented using tree-sitter it shouldn't be too bad,
  - if tree-sitter has an injection that is implemented using a tmLanguage how do i make this work?
  - create a custom tree-sitter language that accepts everything, customise the highlight query at run-time to highlight
    the contents with something like "meta.injection.<injection_name>".  This relies on the language asking for the injection
    being able to locate its end point.
    The replacement of the injection span with its highlighting can then be done as a post-processing step over tree-sitter's
    output.
  - the output iterator would need to be recursive depending on multiple injection depth levels + have look ahead to allow for
    this injection method as any injected language could further inject another one while it is being parsed.
- no planned support for incremental highlighting
- no planned conversion from the output to styles e.g. HTML.  Users could use classes/inline styles/something else, too many
  different ways of doing it to make it worth including in the library
- no syntaxes or themes to be shipped with the library (outside of the tests directory) as i don't want to say whether tmLanguage
  vs sublime-syntax vs tree-sitter is better for any language (or deal with deciding which languages to include)
