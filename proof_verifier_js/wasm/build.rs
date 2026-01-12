fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target == "wasm32-unknown-unknown" {
        println!("cargo:rustc-cfg=getrandom_backend=\"wasm_js\"");
    }
}
