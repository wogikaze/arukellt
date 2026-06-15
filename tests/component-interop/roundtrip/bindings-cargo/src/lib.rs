// Interim binding artifact (#618): wit-bindgen Rust guest generated from emitted WIT.
wit_bindgen::generate!({
    world: "guest",
    path: "wit",
});

struct GuestImpl;

impl Guest for GuestImpl {
    fn run() -> i32 {
        identity(21)
    }
}

export!(GuestImpl);
