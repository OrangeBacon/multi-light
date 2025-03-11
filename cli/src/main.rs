fn main() {
    let file = multi_light::Config::from_yaml(
        "text.json",
        r#"%YAML 1.2
---
name: C
file_extensions: [c, h]
scope: source.c

contexts:
  main:
    - match: \b(if|else|for|while)\b
      scope: keyword.control.c"#,
    )
    .unwrap();

    println!("{file:#?}");
}
