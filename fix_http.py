with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Find the problematic pattern
pattern = b'    if !file_path.exists() {\r\n    }\r\n----\r\n        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\r\n    }\r\n    }'
replacement = b'    if !file_path.exists() {\r\n        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\r\n    }'

if pattern in data:
    data = data.replace(pattern, replacement, 1)
    with open('src-tauri/src/server/http.rs', 'wb') as f:
        f.write(data)
    print("Fixed with CRLF!")
else:
    # Try LF
    pattern2 = b'    if !file_path.exists() {\n    }\n----\n        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\n    }\n    }'
    replacement2 = b'    if !file_path.exists() {\n        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));\n    }'
    if pattern2 in data:
        data = data.replace(pattern2, replacement2, 1)
        with open('src-tauri/src/server/http.rs', 'wb') as f:
            f.write(data)
        print("Fixed with LF!")
    else:
        print("Pattern not found, trying partial matches...")
        idx = data.find(b'    if !file_path.exists() {')
        if idx >= 0:
            print(f"Found at byte {idx}")
            print(repr(data[idx:idx+200]))