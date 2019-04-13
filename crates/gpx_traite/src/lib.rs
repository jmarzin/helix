#[macro_use]
extern crate helix;

ruby! {
    class GpxTraite {
        def hello(s: String) -> String {
            s
        }
    }
}