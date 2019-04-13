#[macro_use]
extern crate helix;

ruby! {
    class GpxTraite {
        def hello(s: String) -> String {
            let s2 = String::from(" Jacques");
            s + &s2
        }
    }
}