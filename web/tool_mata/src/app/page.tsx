"use client"
import { useState } from 'react';
import Image from 'next/image';
import UploadFiles from './components/UploadFiles';
import MyFiles from './components/MyFiles';

interface GitHubUser {
  login: string;
  avatar_url: string;
}

export default function Page() {
  const [currentSection, setCurrentSection] = useState('upload');
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [user, setUser] = useState<GitHubUser | null>(null);

  const handleGitHubLogin = () => {
    if (!process.env.NEXT_PUBLIC_GITHUB_CLIENT_ID) {
      console.error('GitHub Client ID is not configured');
      return;
    }
    
    const clientId = process.env.NEXT_PUBLIC_GITHUB_CLIENT_ID;
    const redirectUri = 'http://localhost:3000/api/auth/callback';
    const scope = 'read:user';
    
    const authUrl = new URL('https://github.com/login/oauth/authorize');
    authUrl.searchParams.append('client_id', clientId);
    authUrl.searchParams.append('redirect_uri', redirectUri);
    authUrl.searchParams.append('scope', scope);
    
    window.location.href = authUrl.toString();
  };

  // Add callback handler
  const handleAuthCallback = async (code: string) => {
    try {
      const response = await fetch('/api/auth/callback?code=' + code);
      const data = await response.json();
      
      if (data.access_token) {
        setIsAuthenticated(true);
        // Fetch user data
        const userResponse = await fetch('https://api.github.com/user', {
          headers: {
            Authorization: `Bearer ${data.access_token}`,
          },
        });
        const userData = await userResponse.json();
        setUser(userData);
      }
    } catch (error) {
      console.error('Authentication error:', error);
    }
  };

  const LoginSection = () => (
    <div className="p-4 border-b border-gray-700">
      {!isAuthenticated ? (
        <button
          onClick={handleGitHubLogin}
          className="flex items-center gap-2 px-4 py-2 w-full text-gray-200 
            bg-[#2D333B] hover:bg-[#444C56] rounded-md transition-colors"
        >
          <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
            <path fillRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.87 8.17 6.84 9.49.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.61.07-.61 1 .07 1.53 1.03 1.53 1.03.89 1.52 2.34 1.08 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.94 0-1.09.39-1.98 1.03-2.68-.1-.25-.45-1.27.1-2.64 0 0 .84-.27 2.75 1.02.8-.22 1.65-.33 2.5-.33.85 0 1.7.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.37.2 2.39.1 2.64.64.7 1.03 1.59 1.03 2.68 0 3.84-2.34 4.68-4.57 4.93.36.31.68.92.68 1.85v2.74c0 .27.18.58.69.48C19.13 20.17 22 16.42 22 12c0-5.523-4.477-10-10-10z" />
          </svg>
          Sign in with GitHub
        </button>
      ) : (
        <div className="flex items-center gap-2 text-gray-200">
          {user?.avatar_url && (
            <Image 
              src={user.avatar_url}
              alt="Profile"
              width={32}
              height={32}
              className="rounded-full"
            />
          )}
          <span className="text-sm">{user?.login}</span>
        </div>
      )}
    </div>
  );



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
        <LoginSection />
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
        {currentSection === 'upload' && <UploadFiles />}
        {currentSection === 'myFiles' && <MyFiles />}
      </div>
    </div>
  );
}