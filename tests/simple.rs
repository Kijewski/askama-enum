#![cfg(feature = "testing")]

use askama_enum::EnumTemplate;

#[derive(EnumTemplate)]
enum MyEnum {
    #[template(ext = "txt", source = "A")]
    A,
    #[template(ext = "txt", source = "B")]
    B,
    #[template(ext = "txt", source = "C")]
    C,
}

#[test]
fn test() {
    assert_eq!(MyEnum::A.to_string(), "A");
    assert_eq!(MyEnum::B.to_string(), "B");
    assert_eq!(MyEnum::C.to_string(), "C");
}
