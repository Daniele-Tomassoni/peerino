with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Find the two occurrences of 'File ricevuto via inbox locale'
idx1 = data.find(b'File ricevuto via inbox locale')
idx2 = data.find(b'File ricevuto via inbox locale', idx1 + 1)
print(f'First at byte: {idx1}')
print(f'Second at byte: {idx2}')
print(f'File size: {len(data)}')

if idx2 >= 0:
    print(f'Context around second: {repr(data[idx2-20:idx2+100])}')

# The dead code starts after the first streaming block (after '});' at line 534)
# and ends before the real return at the end of the function.
# We need to find the exact boundaries.

# First streaming block ends with:     });
# Then blank line
# Then the dead code starts with:     log::info!("File ricevuto via inbox locale...
# The real code continues after the dead code ends with the actual Ok() return.

# Let's find the pattern: '});\r\n\r\n    log::info!("File ricevuto'
# This should be the boundary between first block and dead code
boundary = b'});\r\n\r\n    log::info!("File ricevuto via inbox locale'
bidx = data.find(boundary)
print(f'Boundary at byte: {bidx}')
if bidx >= 0:
    print(f'Context: {repr(data[bidx:bidx+100])}')

# The dead code ends right before the real return
# Real return is:     log::info!("File ricevuto via inbox locale...);\r\n    Ok(Json...
# So we need to find the second occurrence of 'log::info!("File ricevuto'
# and remove everything from the first boundary to just before the second log::info!

# Find first log::info!("File ricevuto
log1 = data.find(b'log::info!("File ricevuto via inbox locale')
log2 = data.find(b'log::info!("File ricevuto via inbox locale', log1 + 1)
print(f'First log at: {log1}')
print(f'Second log at: {log2}')

if log1 >= 0 and log2 >= 0:
    # Remove from log1 to log2 (the dead code block)
    data = data[:log1] + data[log2:]
    print(f'Removed dead code from {log1} to {log2}')
    with open('src-tauri/src/server/http.rs', 'wb') as f:
        f.write(data)
    print('Done')
else:
    print('Could not find both log markers')