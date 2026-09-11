fn main() {
    // Disables Windows resource generation if the icon does not exist
    #[cfg(windows)]
    {
        let icon_path = std::path::Path::new("src-tauri/icons/icon.ico");
        if !icon_path.exists() {
            // Don't run the windows-resource build script if the icon is missing
            println!("cargo:rustc-cfg=skip_windows_resources");
        }
    }
}