// gives the windows executable the Sharkord icon in Explorer and shortcuts. the
// running bridge sets its tray and window icons itself from icon.png
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("could not embed the windows icon");
    }
}
