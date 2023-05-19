use janetrs::{janet_abstract::*, IsJanetAbstract, *};
use jfna::*;

#[derive(Debug)]
struct Abstract(usize);

const ABSTRACT_TYPE: JanetAbstractType = JanetAbstractType {
    name: "abstract\0" as *const str as *const std::ffi::c_char,
    gc: None,
    gcmark: None,
    get: None,
    put: None,
    marshal: None,
    unmarshal: None,
    tostring: None,
    compare: None,
    hash: None,
    next: None,
    call: None,
    length: None,
    bytes: None,
};

impl IsJanetAbstract for Abstract {
    const SIZE: usize = std::mem::size_of::<Self>();

    fn type_info() -> &'static JanetAbstractType {
        &ABSTRACT_TYPE
    }
}

// -> Abstract?

#[jfna]
fn output_abstract() -> Janet {
    Janet::j_abstract(JanetAbstract::new(Abstract(42)))
}

#[jfna]
fn input_abstract(a: &Abstract) -> f64 {
    eprintln!("got: {:?}", a);
    a.0 as f64
}

#[jfna]
fn input_option_abstract(maybe_a: Option<&Abstract>) -> f64 {
    eprintln!("got: {:?}", maybe_a);
    maybe_a.map(|a| a.0).unwrap_or(0) as f64
}

#[jfna]
fn input_option(num: Option<f64>) -> f64 {
    eprintln!("got: {:?}", num);
    num.unwrap_or_default()
}

#[jfna]
fn output_result() -> Result<Janet, String> {
    Ok(Janet::nil())
}

#[test]
fn test0() {
    let mut client = client::JanetClient::init_with_default_env().unwrap();

    client.add_c_fn(env::CFunOptions::new("output-abstract", output_abstract));
    client.add_c_fn(env::CFunOptions::new("input-abstract", input_abstract));
    #[rustfmt::skip]
    client.add_c_fn(env::CFunOptions::new("input-option-abstract",input_option_abstract));
    client.add_c_fn(env::CFunOptions::new("input-option", input_option));
    client.add_c_fn(env::CFunOptions::new("output-result", output_result));

    assert_eq!(client.run("(input-option 42)").unwrap(), Janet::from(42.0));
    assert_eq!(client.run("(input-option)").unwrap(), Janet::from(0.0));

    assert_eq!(client.run("(output-result)").unwrap(), Janet::nil());

    assert_eq!(
        client
            .run("(let (abs (output-abstract)) (input-abstract abs))")
            .unwrap(),
        Janet::from(42.0)
    );

    assert_eq!(
        client
            .run("(let (abs (output-abstract)) (input-option-abstract abs))")
            .unwrap(),
        Janet::from(42.0)
    );

    assert_eq!(
        client
            .run("(let (abs (output-abstract)) (input-option-abstract))")
            .unwrap(),
        Janet::from(0.0)
    );

    assert_eq!(
        client
            .run("(let (abs (output-abstract)) (input-option-abstract nil))")
            .unwrap(),
        Janet::from(0.0)
    );
}
