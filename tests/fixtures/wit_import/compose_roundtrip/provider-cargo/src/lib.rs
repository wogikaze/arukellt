wit_bindgen::generate!({
    world: "host-provider",
    path: "wit",
});

struct Host;

impl exports::test::host::math::Guest for Host {
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}

export!(Host);
