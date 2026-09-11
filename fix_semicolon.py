with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Find all occurrences of into_response() without semicolon
idx = 0
count = 0
while True:
    idx = data.find(b'into_response())', idx)
    if idx == -1:
        break
    # Check what follows
    after = data[idx+16:idx+20]
    print(f'Found at byte {idx}, after: {repr(after)}')
    if after == b'\r\n}' or after == b'\n}':
        print('  -> Already has semicolon (followed by closing brace)')
    elif after == b'\r\n}' or after == b'\n}':
        print('  -> Already fixed')
    else:
        # Check if next char is ;
        next_char = data[idx+16:idx+17]
        if next_char == b';':
            print('  -> Already has semicolon')
        else:
            print('  -> MISSING semicolon!')
            data = data[:idx+16] + b';' + data[idx+16:]
            count += 1
            print('  -> Fixed!')
    idx += 1

print(f'\nTotal fixed: {count}')
with open('src-tauri/src/server/http.rs', 'wb') as f:
    f.write(data)