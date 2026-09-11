import os
import glob

# 1. Apply header changes to all source files
extensions = ['*.rs', '*.ts', '*.html', '*.css']
modified = 0

for ext in extensions:
    for filepath in glob.glob('**/' + ext, recursive=True):
        # Skip target directory and node_modules
        if 'target' in filepath or 'node_modules' in filepath:
            continue
        try:
            with open(filepath, 'rb') as f:
                data = f.read()

            original = data
            data = data.replace(
                b'P2P file sharing senza cloud, senza account, senza intermediari.',
                b'P2P file sharing without cloud, without accounts, without intermediaries.'
            )
            data = data.replace(
                b'Copyright (C) 2025 Daniele',
                b'Copyright (C) 2026 Daniele Tomassoni'
            )

            if data != original:
                with open(filepath, 'wb') as f:
                    f.write(data)
                modified += 1
                print(f'  Modified: {filepath}')
        except Exception as e:
            print(f'  Error with {filepath}: {e}')

print(f'\nTotal source files modified: {modified}')

# 2. Update README.md
with open('README.md', 'rb') as f:
    data = f.read()
data = data.replace(
    b'**P2P file sharing senza cloud, senza account, senza intermediari.**',
    b'**P2P file sharing without cloud, without accounts, without intermediaries.**'
)
with open('README.md', 'wb') as f:
    f.write(data)
print('Updated README.md')

# 3. Update package.json - add author field
with open('package.json', 'rb') as f:
    data = f.read()
old = b'  "version": "3.2.0",\n  "description"'
new = b'  "version": "3.2.0",\n  "author": "Daniele Tomassoni",\n  "description"'
if old in data:
    data = data.replace(old, new, 1)
    with open('package.json', 'wb') as f:
        f.write(data)
    print('Updated package.json')
else:
    print('package.json pattern not found')

# 4. Update Cargo.toml - add authors field
with open('src-tauri/Cargo.toml', 'rb') as f:
    data = f.read()
old = b'version = "3.2.0"\nedition = "2021"'
new = b'version = "3.2.0"\nauthors = ["Daniele Tomassoni"]\nedition = "2021"'
if old in data:
    data = data.replace(old, new, 1)
    with open('src-tauri/Cargo.toml', 'wb') as f:
        f.write(data)
    print('Updated Cargo.toml')
else:
    print('Cargo.toml pattern not found')

# 5. Fix http.rs pre-existing bugs
with open('src-tauri/src/server/http.rs', 'rb') as f:
    data = f.read()

# Fix 1: Brace issue (lines 266-271)
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
    print('Fixed http.rs brace issue')
else:
    print('http.rs brace pattern not found')

# Fix 2: Remove duplicate dead code block
log1 = data.find(b'log::info!("File ricevuto via inbox locale')
log2 = data.find(b'log::info!("File ricevuto via inbox locale', log1 + 1)
if log1 >= 0 and log2 >= 0:
    data = data[:log1] + data[log2:]
    print(f'Removed dead code from {log1} to {log2}')

with open('src-tauri/src/server/http.rs', 'wb') as f:
    f.write(data)
print('http.rs fixes applied')

print('\nAll done!')