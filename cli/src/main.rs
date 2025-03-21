fn main() {
    let file = multi_light::Config::from_json_debug(
        "text.json",
        r#"[{
        "1": {
          "name": "punctuation.definition\".tag"
        },
        "2": {
          "name": "entity.name.tag"
        }
      }]"#,
    )
    .unwrap();

    println!("{file:#?}");
}
