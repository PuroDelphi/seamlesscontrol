fn main() {
    println!("cargo:rerun-if-changed=../manifest.json");
    let manifest =
        std::fs::read_to_string("../manifest.json").expect("read SeamlessControl plugin manifest");
    let version = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("\"version\": \""))
        .and_then(|value| value.split('"').next())
        .filter(|value| !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .expect("read SeamlessControl product version");
    println!("cargo:rustc-env=SEAMLESSCONTROL_PRODUCT_VERSION={version}");
    println!("cargo:rerun-if-changed=assets/seamlesscontrol.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winres::WindowsResource::new()
            .set_icon("assets/seamlesscontrol.ico")
            .compile()
            .expect("embed SeamlessControl icon in Windows executables");
    }
}
