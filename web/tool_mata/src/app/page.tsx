"use client"
import { useEffect, useState } from 'react';

// Add interfaces
interface UserInfo {
  login: string;
  avatar_url: string;
}

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


export default function FileManager() {
  // Add new state for auth
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [userInfo, setUserInfo] = useState<UserInfo | null>(null);

  const [currentSection, setCurrentSection] = useState('myFiles'); // Add this near other state declarations

  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploadedFiles, setUploadedFiles] = useState<FileUploadResponse[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);


  // Add state for mobile menu
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

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

  // Add GitHub login handler
  const handleGitHubLogin = () => {
    // You'll need to register your app with GitHub and get a client ID
    const clientId = 'YOUR_GITHUB_CLIENT_ID';
    const redirectUri = encodeURIComponent('http://localhost:3000/api/auth/callback');
    const scope = 'read:user';

    window.location.href = `https://github.com/login/oauth/authorize?client_id=${clientId}&redirect_uri=${redirectUri}&scope=${scope}`;
  };

  // Check auth status on mount
  useEffect(() => {
    const checkAuth = async () => {
      try {
        const response = await fetch('http://localhost:3000/api/auth/status');
        const data = await response.json();

        if (data.authenticated) {
          setIsLoggedIn(true);
          setUserInfo(data.user);
        }
      } catch (err) {
        console.error('Auth check failed:', err);
      }
    };

    checkAuth();
  }, []);

  return (
    <div className="flex flex-col lg:flex-row h-screen bg-[#0D1117]">
      {/* Mobile Header */}
      <div className="lg:hidden flex items-center justify-between p-4">
        <h1 className="text-xl font-bold text-gray-200">File Manager</h1>
        <button
          onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
          className="p-2 text-gray-400 hover:text-gray-200 focus:outline-none"
        >
          <svg
            className="w-6 h-6 transition-transform duration-200 ease-in-out"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d={isMobileMenuOpen ? "M6 18L18 6M6 6l12 12" : "M4 6h16M4 12h16M4 18h16"}
            />
          </svg>
        </button>
      </div>

      {/* Sidebar */}
      <div
        className={`
          fixed inset-y-0 left-0 transform lg:relative lg:translate-x-0
          ${isMobileMenuOpen ? 'translate-x-0' : '-translate-x-full'}
          transition-transform duration-300 ease-in-out
          w-64 bg-[#0D1117] border-r border-gray-700 
          flex flex-col z-30 lg:z-0
        `}
      >
        {/* Sidebar Header */}
        <div className="p-4">
          <div className="flex items-center gap-3">
            <svg className="w-8 h-8 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"
              />
            </svg>
            <h1 className="text-xl font-bold text-gray-200">File Manager</h1>
          </div>
        </div>

        {/* User Profile Section */}
        <div className="p-4 border-b border-gray-700">
          {isLoggedIn && userInfo ? (
            <div className="flex items-center gap-3">
              <img
                src={userInfo.avatar_url}
                alt={userInfo.login}
                className="w-10 h-10 rounded-full ring-2 ring-gray-700"
              />
              <div className="flex flex-col">
                <span className="text-gray-200 font-medium">{userInfo.login}</span>
                <button
                  onClick={() => {/* Add logout handler */ }}
                  className="text-sm text-gray-400 hover:text-gray-200 transition-colors"
                >
                  Sign out
                </button>
              </div>
            </div>
          ) : (
            <button
              onClick={handleGitHubLogin}
              className="flex items-center justify-center gap-2 w-full px-4 py-2 
                bg-gray-800 hover:bg-gray-700 rounded-md transition-colors"
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
              </svg>
              Sign in with GitHub
            </button>
          )}
        </div>

        {/* Navigation */}
        <nav className="flex-1 p-4 space-y-2">
          <button
            onClick={() => setCurrentSection('upload')}
            className={`flex items-center gap-2 px-4 py-2 text-gray-200 
      hover:bg-gray-800 rounded-md transition-colors group w-full text-left
      ${currentSection === 'upload' ? 'bg-gray-800' : ''}`}
          >
            <svg
              className="w-5 h-5 text-gray-400 group-hover:text-blue-400 transition-colors"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
              />
            </svg>
            Upload Files
          </button>
          <button
            onClick={() => setCurrentSection('myFiles')}
            className={`flex items-center gap-2 px-4 py-2 text-gray-200 
      hover:bg-gray-800 rounded-md transition-colors group w-full text-left
      ${currentSection === 'myFiles' ? 'bg-gray-800' : ''}`}
          >
            <svg
              className="w-5 h-5 text-gray-400 group-hover:text-blue-400 transition-colors"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"
              />
            </svg>
            My Files
          </button>
        </nav>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-auto w-full">
        <div className="max-w-3xl mx-auto p-4 lg:p-6 space-y-4 lg:space-y-6">
          {/* File Upload Section */}
          <div
            className="border-2 border-dashed border-gray-700 rounded-lg p-6 bg-[#161B22] hover:border-blue-500 transition-colors"
            onDragOver={(e) => {
              e.preventDefault();
              e.stopPropagation();
              e.currentTarget.classList.add('border-blue-500');
            }}
            onDragLeave={(e) => {
              e.preventDefault();
              e.stopPropagation();
              e.currentTarget.classList.remove('border-blue-500');
            }}
            onDrop={(e) => {
              e.preventDefault();
              e.stopPropagation();
              e.currentTarget.classList.remove('border-blue-500');
              if (e.dataTransfer.files && e.dataTransfer.files[0]) {
                setSelectedFile(e.dataTransfer.files[0]);
                setError(null);
              }
            }}
          >
            <input
              type="file"
              onChange={handleFileSelect}
              className="hidden"
              id="fileInput"
            />
            <label
              htmlFor="fileInput"
              className="flex flex-col items-center justify-center gap-4 cursor-pointer"
            >
              <div className="p-4 bg-blue-500/10 rounded-full">
                <svg
                  className="w-8 h-8 text-blue-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                  />
                </svg>
              </div>
              <div className="text-center">
                <p className="text-gray-200 font-medium">
                  {selectedFile ? selectedFile.name : 'Drop files here or click to upload'}
                </p>
                <p className="mt-1 text-sm text-gray-400">
                  {selectedFile
                    ? `${(selectedFile.size / 1024).toFixed(2)} KB`
                    : 'Supports all file types'}
                </p>
              </div>
            </label>
          </div>

          {/* Upload Button */}
          <button
            onClick={uploadFile}
            disabled={!selectedFile || loading}
            className={`w-full py-2 px-4 rounded-md text-white font-medium ${!selectedFile || loading
              ? 'bg-gray-700 cursor-not-allowed'
              : 'bg-blue-600 hover:bg-blue-700'
              } transition-colors`}
          >
            {loading ? 'Uploading...' : 'Upload File'}
          </button>

          {/* Error Message */}
          {error && (
            <div className="text-red-400 text-sm bg-red-900/20 p-3 rounded-md">
              {error}
            </div>
          )}

          {/* Uploaded Files List */}
          {uploadedFiles.length > 0 && (
            <div className="space-y-4">
              <h2 className="text-lg font-semibold text-gray-200">Uploaded Files</h2>
              <div className="space-y-3">
                {uploadedFiles.map((file) => (
                  <div
                    key={file.id}
                    className="flex flex-col sm:flex-row sm:items-center justify-between p-4 bg-[#161B22] rounded-lg border border-gray-700 hover:border-gray-600 transition-colors gap-4"
                  >
                    <div className="flex items-center gap-3">
                      <div className="p-2 bg-blue-500/10 rounded-lg shrink-0">
                        <svg className="w-6 h-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                          />
                        </svg>
                      </div>
                      <div className="min-w-0">
                        <h3 className="text-gray-200 font-medium truncate">{file.filename}</h3>
                        <p className="text-sm text-gray-400 truncate">
                          {(file.size / 1024).toFixed(2)} KB • ID: {file.id}
                        </p>
                      </div>
                    </div>
                    <button
                      onClick={() => window.open(file.location)}
                      className="flex items-center justify-center gap-2 px-3 py-1.5 text-sm text-blue-400 hover:text-blue-300 transition-colors w-full sm:w-auto"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                          d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                        />
                      </svg>
                      Download
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Download by ID Section */}
          <div className="border border-gray-700 rounded-lg p-4 lg:p-6 bg-[#161B22]">
            <h2 className="text-lg font-semibold text-gray-200 mb-4">Download by ID</h2>
            <div className="space-y-4">
              <div className="flex flex-col sm:flex-row gap-3">
                <input
                  type="text"
                  value={downloadState.fileId}
                  onChange={(e) => setDownloadState(prev => ({ ...prev, fileId: e.target.value }))}
                  placeholder="Enter file ID"
                  className="flex-1 px-4 py-2 bg-[#0D1117] border border-gray-700 rounded-lg 
                  text-gray-200 placeholder-gray-500 focus:border-blue-500 focus:ring-1 
                  focus:ring-blue-500 outline-none transition-colors"
                />
                <button
                  onClick={handleFileDownload}
                  disabled={downloadState.isDownloading || !downloadState.fileId}
                  className={`px-4 py-2 rounded-lg text-white font-medium flex items-center gap-2
                    ${downloadState.isDownloading || !downloadState.fileId
                      ? 'bg-gray-700 cursor-not-allowed'
                      : 'bg-blue-600 hover:bg-blue-700'
                    } transition-colors`}
                >
                  {downloadState.isDownloading ? (
                    <>
                      <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                      </svg>
                      Downloading...
                    </>
                  ) : (
                    <>
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                          d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                        />
                      </svg>
                      Download
                    </>
                  )}
                </button>
              </div>
              {downloadState.error && (
                <div className="text-red-400 text-sm bg-red-900/20 p-3 rounded-lg">
                  {downloadState.error}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}