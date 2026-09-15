fn main() {
    let mut attributes = tauri_build::Attributes::new();

    #[cfg(windows)]
    {
        // Use our custom manifest to ensure Common Controls v6 support,
        // which fixes STATUS_ENTRYPOINT_NOT_FOUND (TaskDialogIndirect).
        attributes = attributes.windows_attributes(
            tauri_build::WindowsAttributes::new()
                .app_manifest(include_str!("windows-app.manifest")),
        );
    }

    tauri_build::try_build(attributes).expect("tauri-build failed");
}