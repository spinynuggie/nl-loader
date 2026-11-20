fn main() {
    println!("cargo:rerun-if-changed=assets/icons/app.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icons/app.ico");
        resource.set("ProductName", "Neverlose Launcher");
        resource.set("FileDescription", "Neverlose Launcher");
        resource.compile().expect("failed to compile Windows resources");
    }
}
