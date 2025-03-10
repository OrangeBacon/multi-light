fn main() {
    let file = multi_light::Config::from_json(
        "text.json",
        r#"{
        "a": [1,2]
    }"#,
    )
    .unwrap();

    println!("{file:#?}");
}
