#[macro_use]
extern crate helix;

ruby! {
    class GpxTraite {
        def hello() -> String {
            String::from("Hello from gpx_traite!")
        }
    }
}