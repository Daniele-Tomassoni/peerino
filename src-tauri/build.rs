fn main() {
    // Disabilita la generazione delle risorse Windows se l'icona non esiste
    #[cfg(windows)]
    {
        // Non generare risorse Windows
        println!("cargo:rustc-env=TAURI_ICON_PATH=");
    }
    
    tauri_build::build();
}