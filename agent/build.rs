fn main() {
    println!("cargo:rerun-if-changed=assets/seamlesscontrol.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winres::WindowsResource::new()
            .set_icon("assets/seamlesscontrol.ico")
            .compile()
            .expect("embed SeamlessControl icon in Windows executables");
    }
}
