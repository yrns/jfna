use janetrs::*;
use jfna::*;

#[jfna]
fn test(num: f64, string: JanetString) -> Result<f64, String> {
    eprintln!("got: {}, {}", num, string);
    Ok(42.0)
}

#[test]
fn test0() {
    let mut client = client::JanetClient::init_with_default_env().unwrap();

    client.add_c_fn(env::CFunOptions::new("test", test));

    assert_eq!(client.run("(test 1.0 \"xyz\")").unwrap(), Janet::from(42.0));
}
