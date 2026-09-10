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
pub mod network;
pub mod temp_cleanup;
pub mod peer_id;
pub mod turn_creds;
pub mod ice_provider;

/// Verifica che un filename sia sicuro (no path traversal).
/// Rifiuta nomi vuoti o contenenti `..`, `/` o `\`.
/// Condiviso tra i vari handler per evitare duplicazioni e mantenere
/// coerente la validazione dei nomi file in tutto il backend.
pub fn is_safe_filename(filename: &str) -> bool {
    !filename.is_empty()
        && !filename.contains("..")
        && !filename.contains('/')
        && !filename.contains('\\')
}

/// Restituisce la directory in cui si trova l'eseguibile dell'app.
/// Questo garantisce che le cartelle `shared-folder`, `temp` e `config`
/// vengano create nella stessa directory di `peerino.exe`, indipendentemente
/// dalla directory di lavoro corrente.
pub fn get_app_dir() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}
