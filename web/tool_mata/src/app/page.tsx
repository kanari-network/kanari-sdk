"use client"
import { useState } from 'react';

interface FileUploadResponse {
  id: string;
  filename: string;
  location: string;
  size: number;
  content_type: string;
}

interface FileDownloadState {
  isDownloading: boolean;
  fileId: string;
  error: string | null;
}

interface FileGetResponse extends FileUploadResponse {
  data: string;
}

export default function FileManager() {
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploadedFiles, setUploadedFiles] = useState<FileUploadResponse[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [downloadState, setDownloadState] = useState<FileDownloadState>({
    isDownloading: false,
    fileId: '',
    error: null
  });

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setSelectedFile(e.target.files[0]);
      setError(null);
    }
  };

  const uploadFile = async () => {
    if (!selectedFile) {
      setError('Please select a file');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      // Convert file to base64
      const base64 = await new Promise<string>((resolve) => {
        const reader = new FileReader();
        reader.onloadend = () => {
          const base64 = reader.result as string;
          resolve(base64.split(',')[1]);
        };
        reader.readAsDataURL(selectedFile);
      });

      // Make RPC call
      const response = await fetch('http://localhost:3000/api/rpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'upload_file',
          params: {
            filename: selectedFile.name,
            data: base64
          },
          id: 1
        })
      });

      const result = await response.json();
      if (result.error) {
        throw new Error(result.error.message);
      }

      setUploadedFiles(prev => [...prev, result.result]);
      setSelectedFile(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setLoading(false);
    }
  };

  const handleFileDownload = async () => {
    if (!downloadState.fileId.trim()) {
      setDownloadState(prev => ({ ...prev, error: 'Please enter a file ID' }));
      return;
    }

    setDownloadState(prev => ({ ...prev, isDownloading: true, error: null }));

    try {
      const response = await fetch('http://localhost:3000/api/rpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'get_file',
          params: downloadState.fileId,
          id: 1
        })
      });

      const result = await response.json();
      if (result.error) {
        throw new Error(result.error.message);
      }

      // Create a download link for the file
      const fileData = result.result.data;
      const blob = new Blob([Buffer.from(fileData, 'base64')], {
        type: result.result.content_type
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = result.result.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      setDownloadState(prev => ({
        ...prev,
        fileId: '',
        error: null
      }));
    } catch (err) {
      setDownloadState(prev => ({
        ...prev,
        error: err instanceof Error ? err.message : 'Download failed'
      }));
    } finally {
      setDownloadState(prev => ({ ...prev, isDownloading: false }));
    }
  };

  return (
    <div className="w-full max-w-2xl mx-auto p-6 space-y-6">
      <div className="border-2 border-dashed border-gray-300 rounded-lg p-6">
        <input
          type="file"
          onChange={handleFileSelect}
          className="hidden"
          id="fileInput"
        />
        <label
          htmlFor="fileInput"
          className="flex flex-col items-center justify-center cursor-pointer"
        >
          <svg
            className="w-12 h-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 4v16m8-8H4"
            />
          </svg>
          <span className="mt-2 text-sm text-gray-600">
            {selectedFile ? selectedFile.name : 'Select a file'}
          </span>
        </label>
      </div>

      <button
        onClick={uploadFile}
        disabled={!selectedFile || loading}
        className={`w-full py-2 px-4 rounded-md text-white ${
          !selectedFile || loading
            ? 'bg-gray-400'
            : 'bg-blue-500 hover:bg-blue-600'
        }`}
      >
        {loading ? 'Uploading...' : 'Upload File'}
      </button>

      {error && (
        <div className="text-red-500 text-sm">{error}</div>
      )}

      {uploadedFiles.length > 0 && (
        <div className="mt-6">
          <h3 className="text-lg font-semibold mb-4">Uploaded Files</h3>
          <div className="space-y-2">
            {uploadedFiles.map((file) => (
              <div
                key={file.id}
                className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
              >
                <div>
                  <p className="font-medium">{file.filename}</p>
                  <p className="text-sm text-gray-500">
                    {(file.size / 1024).toFixed(2)} KB
                  </p>
                </div>
                <button
                  onClick={() => window.open(file.location)}
                  className="text-blue-500 hover:text-blue-600"
                >
                  Download
                </button>
              </div>
            ))}
          </div>
        </div>
      )}


<div className="border rounded-lg p-6 bg-gray-50">
        <h3 className="text-lg font-semibold mb-4">Download File by ID</h3>
        <div className="flex gap-2">
          <input
            type="text"
            value={downloadState.fileId}
            onChange={(e) => setDownloadState(prev => ({ ...prev, fileId: e.target.value }))}
            placeholder="Enter file ID"
            className="flex-1 p-2 border rounded-md"
          />
          <button
            onClick={handleFileDownload}
            disabled={downloadState.isDownloading || !downloadState.fileId}
            className={`px-4 py-2 rounded-md text-white ${
              downloadState.isDownloading || !downloadState.fileId
                ? 'bg-gray-400'
                : 'bg-blue-500 hover:bg-blue-600'
            }`}
          >
            {downloadState.isDownloading ? 'Downloading...' : 'Download'}
          </button>
        </div>
        {downloadState.error && (
          <div className="mt-2 text-red-500 text-sm">{downloadState.error}</div>
        )}
      </div>
    </div>
  );
}