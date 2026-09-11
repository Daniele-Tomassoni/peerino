with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Remove the premature Ok() return at line 537 that's followed by dead code
# The actual return is at the end of the function (~line 696)
old = (b'    log::info!(\"File ricevuto via inbox locale: {} (hash: {})\", final_filename, hash);\r\n'
       b'    Ok(Json(json!({\"status\": \"ok\", \"hash\": hash, \"filename\": final_filename})).into_response());\r\n'
       b'    // cosi il pulsante')
new = (b'    log::info!(\"File ricevuto via inbox locale: {} (hash: {})\", final_filename, hash);\r\n'
       b'    // cosi il pulsante')

if old in data:
    data = data.replace(old, new, 1)
    print('Removed premature Ok()')
else:
    print('Pattern not found')
    # Debug
    idx = data.find(b'File ricevuto via inbox locale')
    if idx >= 0:
        print(f'Context: {repr(data[idx:idx+200])}')

with open('src-tauri/src/server/http.rs', 'wb') as f:
    f.write(data)
print('Done')