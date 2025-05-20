fn main() {
    let file = multi_light::Config::from_json(
        "text.json",
        r#"[{
        "1": {
          "name": "punctuation.definition.tag"
        },
        "2": {
          "name": "entity.name.tag"
        }
      }, 5.4, {}, ["a", "b"]]"#,
    )
    .unwrap();

    println!("{file:#?}");
}
