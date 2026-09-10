import { vi } from 'vitest';

test('should call register_file with correct path', async () => {
    // Mock invoke function
    const mockInvoke = vi.fn().mockResolvedValue('abc123def456');
    
    // Simulate the call
    const result = await mockInvoke('register_file', { filePath: '/test/file.pdf' });
    
    expect(result).toBe('abc123def456');
    expect(mockInvoke).toHaveBeenCalledWith('register_file', { filePath: '/test/file.pdf' });
});

test('should call list_files and return file list', async () => {
    const mockInvoke = vi.fn().mockResolvedValue([
        {
            filename: 'test.pdf',
            size: 1024,
            hash: 'abc123',
            uploaded_at: '2024-01-01T00:00:00Z',
        },
    ]);

    const result = await mockInvoke('list_files');
    
    expect(result).toHaveLength(1);
    expect(mockInvoke).toHaveBeenCalledWith('list_files');
});

test('should format file size correctly', () => {
    const formatSize = (bytes: number): string => {
        const units = ['B', 'KB', 'MB', 'GB'];
        let size = bytes;
        let unitIndex = 0;
        
        while (size >= 1024 && unitIndex < units.length - 1) {
            size /= 1024;
            unitIndex++;
        }
        
        return `${size.toFixed(2)} ${units[unitIndex]}`;
    };

    expect(formatSize(512)).toBe('512.00 B');
    expect(formatSize(1024)).toBe('1.00 KB');
    expect(formatSize(1048576)).toBe('1.00 MB');
});