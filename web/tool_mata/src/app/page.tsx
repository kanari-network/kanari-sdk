"use client"
import { useEffect, useState } from 'react';
import UploadFiles from './components/UploadFiles';
import MyFiles from './components/MyFiles';


export default function FileManager() {


  const [currentSection, setCurrentSection] = useState('upload'); // Add this near other state declarations

  // Add state for mobile menu
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);


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