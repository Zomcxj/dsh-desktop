#[cfg(target_os = "windows")]
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    winres::WindowsResource::new()
        .set_icon("assets/icon.ico")
        .compile()
        .expect("embed Windows executable icon");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
