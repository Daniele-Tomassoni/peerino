with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Find the pattern and fix it
old = b')).into_response())\n}'
new = b')).into_response();\n}'

count = data.count(old)
print(f"Found {count} occurrences")

if count > 0:
    data = data.replace(old, new, 1)
    with open('src-tauri/src/server/http.rs', 'wb') as f:
        f.write(data)
    print("Fixed!")
else:
    # Try to find it differently
    idx = data.find(b'into_response())')
    if idx >= 0:
        print(f"Found 'into_response())' at byte {idx}")
        print(repr(data[idx:idx+60]))