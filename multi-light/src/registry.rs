/// Storage for all data required to syntax highlight a piece of source code
#[derive(Debug, Clone, Copy)]
pub struct Registry<F> {
    /// function to use to read a file referenced from a source file
    callback: Option<F>,
}

impl<F> Registry<F> {
    /// Create a new registry, containing no syntaxes or grammars
    pub fn new() -> Self {
        Registry { callback: None }
    }
}

impl<F> Default for Registry<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F, E> Registry<F>
where
    F: FnMut(&str) -> Result<(), E>,
{
    // fn read_file(f: fn(&str) -> Result<impl HighlightInput>) {} // function for reading a required dependency file
    // fn add(name: &str, input: impl HighlightInput) -> Result<()> {} // add a file to the registry
    // // Get the theme for a given name (or default if there isn't one already).  Allows for more complex construction of themes, i.e.
    // // if you want to merge them, read them, modify them based on code, etc. (do the same for grammars)
    // fn theme(name: &str) -> Theme<'a> {}
    // fn syntax(name: &str) -> Syntax<'a> {}

    // // parse the document.  If using the mut parser, allow calling the read_file callback, otherwise the registry cannot
    // // be changed, so return an error if a file is not found.
    // // allow for language detection based on the available grammars, or, provide a language name
    // fn parse(&self, input: &str) -> output {}
    // fn parse_mut(&mut self, input: &str) -> output {}
}
