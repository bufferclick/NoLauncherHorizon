//! Fabric n Horizon's Windows resources: the icon (`icons/fh.ico`, drawn by
//! `icons/make_icon.py` from `icons/fh.svg`) and the version Explorer shows, both from `VERSION`
//! so the .exe, `--version` and the release tag (`fh-v1.2`) cannot drift apart.
//!
//! `VERSION` also becomes `FH_VERSION`, which `main.rs` reads: one place to bump an update.
fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let version = std::fs::read_to_string(format!("{dir}/VERSION"))
        .map(|text| text.trim().to_owned())
        .unwrap_or_default();
    if version.is_empty() {
        panic!("{dir}/VERSION is empty: it holds the release, such as 1.2");
    }
    println!("cargo:rustc-env=FH_VERSION={version}");
    println!("cargo:rerun-if-changed={dir}/VERSION");
    let ico = format!("{dir}/icons/fh.ico");
    println!("cargo:rerun-if-changed={ico}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Resolve imports before main from System32, never a nearby injector DLL (#13108).
        println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(&ico);
        resource.set("ProductName", "Fabric n Horizon");
        resource.set("FileDescription", "Fabric n Horizon: your Horizon instances, one page");
        resource.set("OriginalFilename", "FH.exe");
        resource.set("FileVersion", &version);
        resource.set("ProductVersion", &version);
        resource.set(
            "LegalCopyright",
            "MIT licence; Horizon is not an official Minecraft product (Mojang/Microsoft)",
        );
        if let Err(e) = resource.compile() {
            panic!("embedding {ico}: {e}");
        }
    }
}
