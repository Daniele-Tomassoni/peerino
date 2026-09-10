test('should format file sizes correctly', () => {
    const formatSize = (bytes: number): string => {
        const units = ['B', 'KB', 'MB', 'GB', 'TB'];
        if (bytes === 0) return '0 B';
        const k = 1024;
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        const size = bytes / Math.pow(k, i);
        return `${size.toFixed(1)} ${units[i]}`;
    };

    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(512)).toBe('512.0 B');
    expect(formatSize(1024)).toBe('1.0 KB');
    expect(formatSize(1048576)).toBe('1.0 MB');
    expect(formatSize(1073741824)).toBe('1.0 GB');
});

test('should format dates correctly', () => {
    const formatDate = (dateString: string): string => {
        try {
            const date = new Date(dateString);
            if (isNaN(date.getTime())) return 'Data sconosciuta';
            return date.toLocaleDateString('it-IT', {
                day: '2-digit',
                month: '2-digit',
                year: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
            });
        } catch {
            return 'Data sconosciuta';
        }
    };

    expect(formatDate('2024-03-08T10:30:00Z')).not.toBe('Data sconosciuta');
    expect(formatDate('invalid-date')).toBe('Data sconosciuta');
});

test('should generate correct download link', () => {
    const networkInfo = { ip: '192.168.1.100', port: 3000 };
    const hash = 'abc123def456';
    
    const link = `http://${networkInfo.ip}:${networkInfo.port}/download/${hash}`;
    
    expect(link).toBe('http://192.168.1.100:3000/download/abc123def456');
});