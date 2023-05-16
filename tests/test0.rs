use janetrs::*;
use jfna::*;

#[jfna]
fn test(num: Option<f64>) -> f64 {
    eprintln!("got: {:?}", num);
    42.0
}

#[test]
fn test0() {
    let mut client = client::JanetClient::init_with_default_env().unwrap();

    client.add_c_fn(env::CFunOptions::new("test", test));

    assert_eq!(client.run("(test nil)").unwrap(), Janet::from(42.0));
}
