wit_bindgen::generate!({
    world: "runner",
    path: "wit",
});

struct Runner;

impl Guest for Runner {
    fn run() -> i32 {
        add(40, 2)
    }
}

export!(Runner);
