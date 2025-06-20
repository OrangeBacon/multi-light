fn main() {
    let mut registry = multi_light::Registry::new();
    registry.add("my_theme", "input").unwrap();
    registry.add("my_syntax", "data").unwrap();
}
