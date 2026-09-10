// Peerino - P2P file sharing senza cloud, senza account, senza intermediari.
// Copyright (C) 2025 Daniele
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// Clipboard operations for P2P Share
// Used for copying links and text to clipboard

/// Copy text to clipboard
#[allow(dead_code)]
pub async fn copy_to_clipboard(_text: String) -> Result<(), String> {
    // In Tauri 2.0, clipboard operations are handled by the plugin
    // This is a placeholder for future use
    Ok(())
}

/// Read text from clipboard
#[allow(dead_code)]
pub async fn read_clipboard() -> Result<String, String> {
    // In Tauri 2.0, clipboard operations are handled by the plugin
    // This is a placeholder for future use
    Ok(String::new())
}