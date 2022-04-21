use askama_enum::EnumTemplate;

#[derive(EnumTemplate)]
#[template(ext = "txt", source = "DEFAULT")]
enum MyEnum<'a, T>
where
    T: std::fmt::Display,
{
    A,
    #[template(ext = "html", source = "x{{self.0}}y")]
    B(&'a T),
    #[template(ext = "txt", source = "{{some}}|{{more}}|{{fields}}")]
    C {
        some: T,
        more: u32,
        fields: &'a str,
    },
}

#[test]
fn test() {
    assert_eq!(MyEnum::A::<String>.to_string(), "DEFAULT");
    assert_eq!(MyEnum::B(&"<hello>").to_string(), "x&lt;hello&gt;y");
    assert_eq!(
        MyEnum::C {
            some: "<hello>",
            more: 123,
            fields: "bye",
        }
        .to_string(),
        "<hello>|123|bye"
    );
}
