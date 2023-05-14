use janetrs::*;
use jfna::*;

#[jfna]
fn test(num: f64, string: JanetString) -> Janet {
    println!("got: {}, {}", num, string);
    Janet::nil()
}

#[test]
fn test0() {
    // TODO round trip from janet
}
