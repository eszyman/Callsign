#[allow(dead_code)]
#[path = "src/icon_file.rs"]
mod icon_file;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/icon_file.rs");
    println!("cargo:rerun-if-changed=assets/icon.png");
    embed_windows_icon();
}

#[cfg(target_os = "windows")]
fn embed_windows_icon() {
    let (width, height, rgba) =
        icon_file::decode_png(include_bytes!("assets/icon.png")).expect("assets/icon.png");
    let ico_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("callsign.ico");
    icon_file::write_ico(&ico_path, &rgba, width, height).expect("write the exe icon");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("icon path"));
    res.compile().expect("embed the icon into the exe");
}

#[cfg(not(target_os = "windows"))]
fn embed_windows_icon() {}
