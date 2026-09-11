with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# 1. Header changes
data = data.replace(
    b'P2P file sharing senza cloud, senza account, senza intermediari.',
    b'P2P file sharing without cloud, without accounts, without intermediaries.'
)
data = data.replace(
    b'Copyright (C) 2025 Daniele',
    b'Copyright (C) 2026 Daniele Tomassoni'
)

# 2. Fix pre-existing brace issue (lines 266-271)
old_brace = (
    b'    if !file_path.exists() {\r\n'
    b'    }\r\n'
    b'----\r\n'
    b'        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\r\n'
    b'    }\r\n'
    b'    }'
)
new_brace = (
    b'    if !file_path.exists() {\r\n'
    b'        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\r\n'
    b'    }'
)
if old_brace in data:
    data = data.replace(old_brace, new_brace, 1)
    print('Brace fix applied')
else:
    print('Brace pattern NOT found')

# 3. Fix missing semicolon at the Ok() followed by comment (not closing brace)
old_semi = (
    b'    Ok(Json(json!({"status": "ok", "hash": hash, "filename": final_filename})).into_response())\r\n'
    b'    // cosi il pulsante'
)
new_semi = (
    b'    Ok(Json(json!({"status": "ok", "hash": hash, "filename": final_filename})).into_response());\r\n'
    b'    // cosi il pulsante'
)
if old_semi in data:
    data = data.replace(old_semi, new_semi, 1)
    print('Semicolon fix applied')
else:
    print('Semicolon pattern NOT found')

with open('src-tauri/src/server/http.rs', 'wb') as f:
    f.write(data)
print('Done')