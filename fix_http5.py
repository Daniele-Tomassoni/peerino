with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Find all into_response() occurrences and check what follows
idx = 0
while True:
    idx = data.find(b'into_response())', idx)
    if idx == -1:
        break
    after = data[idx+16:idx+20]
    print(f'Found at byte {idx}, after: {repr(after)}')
    if after[0:1] != b';':
        # Add semicolon
        data = data[:idx+16] + b';' + data[idx+16:]
        print('  -> Added semicolon')
    idx += 1

with open('src-tauri/src/server/http.rs', 'wb') as f:
    f.write(data)
print('Done')