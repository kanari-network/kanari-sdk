import { useState } from 'react';

interface FileItem {
  id: string;
  name: string;
  size: string;
  modified: string;
  type: string;
}

export default function MyFiles() {
  const [files] = useState<FileItem[]>([
    {
      id: '1',
      name: 'document.pdf',
      size: '2.5 MB',
      modified: '2024-02-11',
      type: 'pdf'
    },
    // Add more sample files as needed
  ]);

  return (
    <div className="p-6">
      <h2 className="text-2xl font-bold text-gray-200 mb-6">My Files</h2>
      
      <div className="bg-gray-800 rounded-lg overflow-hidden">
        <div className="grid grid-cols-12 gap-4 p-4 border-b border-gray-700 text-gray-400 text-sm">
          <div className="col-span-6">Name</div>
          <div className="col-span-2">Size</div>
          <div className="col-span-3">Modified</div>
          <div className="col-span-1">Actions</div>
        </div>

        {files.map((file) => (
          <div key={file.id} className="grid grid-cols-12 gap-4 p-4 border-b border-gray-700 hover:bg-gray-700/50 transition-colors">
            <div className="col-span-6 flex items-center gap-2">
              <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
              <span className="text-gray-200">{file.name}</span>
            </div>
            <div className="col-span-2 text-gray-400 flex items-center">{file.size}</div>
            <div className="col-span-3 text-gray-400 flex items-center">{file.modified}</div>
            <div className="col-span-1 flex items-center">
              <button className="text-gray-400 hover:text-blue-400 transition-colors">
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                    d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"
                  />
                </svg>
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}