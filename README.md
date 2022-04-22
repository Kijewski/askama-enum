## askama-enum

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Kijewski/askama-enum/CI?logo=github)](https://github.com/Kijewski/askama-enum/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/askama-enum?logo=rust)](https://crates.io/crates/askama-enum)
![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.53+-important?logo=rust "Minimum Supported Rust Version")
![License](https://img.shields.io/badge/license-ISC%2FMIT%2FApache--2.0%20WITH%20LLVM--exception-informational?logo=apache)

Implement different [Askama](https://crates.io/crates/askama) templates for different enum variants.

```rust
#[derive(EnumTemplate)]
#[template(ext = "html", source = "default")] // default, optional
enum MyEnum<'a, T: std::fmt::Display> {
    // uses the default `#[template]`
    A,

    // uses specific `#[template]`
    #[template(ext = "html", source = "B")]
    B,

    // you can use tuple structs
    #[template(
        ext = "html",
        source = "{{self.0}} {{self.1}} {{self.2}} {{self.3}}",
    )]
    C(u8, &'a u16, u32, &'a u64),

    // and named fields, too
    #[template(ext = "html", source = "{{some}} {{fields}}")]
    D { some: T, fields: T },
}

assert_eq!(
    MyEnum::A::<&str>.to_string(),
    "default",
);
assert_eq!(
    MyEnum::B::<&str>.to_string(),
    "B",
);
assert_eq!(
    MyEnum::C::<&str>(1, &2, 3, &4).to_string(),
    "1 2 3 4",
);
assert_eq!(
    MyEnum::D { some: "some", fields: "fields" }.to_string(),
    "some fields",
);
```
