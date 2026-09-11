with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Apply header changes
data = data.replace(b'P2P file sharing senza cloud, senza account, senza intermediari.',
                     b'P2P file sharing without cloud, without accounts, without intermediaries.')
data = data.replace(b'Copyright (C) 2025 Daniele',
                     b'Copyright (C) 2026 Daniele Tomassoni')

# Fix the pre-existing brace issue
idx = data.find(b'if !file_path.exists()')
print(f'Found at byte {idx}')
print(f'Context: {repr(data[idx:idx+120])}')

# Build the old/new patterns manually
old = (b'    if !file_path.exists() {\r\n'
       b'    }\r\n'
       b'----\r\n'
       b'        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\r\n'
       b'    }\r\n'
       b'    }')
new = (b'    if !file_path.exists() {\r\n'
       b'        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\r\n'
       b'    }')

if old in data:
    data = data.replace(old, new, 1)
    print('Brace fix applied!')
else:
    print('Pattern not found with CRLF, trying LF...')
    old_lf = old.replace(b'\r\n', b'\n')
    new_lf = new.replace(b'\r\n', b'\n')
    if old_lf in data:
        data = data.replace(old_lf, new_lf, 1)
        print('Brace fix applied with LF!')
    else:
        print('Still not found, checking bytes around...')
        print(repr(data[idx:idx+150]))

with open('src-tauri/src/server/http.rs', 'wb') as f:
    f.write(data)
print('Done')