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

// #[jfna]
// fn input_option_abstract(maybe_a: Option<&Abstract>) -> Janet {
//     eprintln!("got: {:?}", maybe_a);
//     Janet::nil()
// }

// #[jfna]
// fn input_option(num: Option<f64>) -> f64 {
//     eprintln!("got: {:?}", num);
//     42.0
// }

// #[jfna]
// fn output_error() -> Result<(), String> {
//     Err("nope".to_owned())
// }

#[test]
fn test0() {
    let mut client = client::JanetClient::init_with_default_env().unwrap();

    client.add_c_fn(env::CFunOptions::new("output-abstract", output_abstract));
    client.add_c_fn(env::CFunOptions::new("input-abstract", input_abstract));
    // client.add_c_fn(env::CFunOptions::new(
    //     "input_option_abstract",
    //     input_option_abstract,
    // ));
    // client.add_c_fn(env::CFunOptions::new("input-option", input_option));
    // client.add_c_fn(env::CFunOptions::new("output_error", output_error));

    assert_eq!(
        client
            .run("(let (abs (output-abstract)) (input-abstract abs))")
            .unwrap(),
        Janet::from(42.0)
    );
}
