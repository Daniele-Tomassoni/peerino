with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

old = b')).into_response())\r\n}\r\n'
new = b')).into_response();\r\n}\r\n'

count = data.count(old)
print(f'Found {count} occurrences')

if count > 0:
    data = data.replace(old, new, 1)
    with open('src-tauri/src/server/http.rs', 'wb') as f:
        f.write(data)
    print('Fixed!')
else:
    print('Pattern not found')
    # Debug
    idx = data.find(b'into_response())')
    if idx >= 0:
        print(f'Context: {repr(data[idx:idx+30])}')