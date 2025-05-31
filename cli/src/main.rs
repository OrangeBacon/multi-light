fn main() {
    let file = multi_light::Config::from_plist(
        "text.plist",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>fileTypes</key>
	<array>
		<string>Makefile</string>
		<string>makefile</string>
		<string>GNUmakefile</string>
		<string>OCamlMakefile</string>
		<true/>
		<false/>
		<integer>57</integer>
		<real>23.4</real>
	</array>
	<key>scopeName</key>
	<string>source.makefile</string>
	<key>uuid</key>
	<string>FF1825E8-6B1C-11D9-B883-000D93589AF6</string>
</dict>
</plist>"#,)
    .unwrap();

    println!("{file:#?}");
}
