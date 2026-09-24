use tauri::State;
use crate::AppState;

#[tauri::command]
pub async fn get_startup_error(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !state.startup_ready.load(std::sync::atomic::Ordering::SeqCst) {
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Ok(state.startup_error.lock().await.clone())
}
