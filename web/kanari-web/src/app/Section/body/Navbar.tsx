'use client'

import Link from "next/link";
import React, { useEffect, useState } from "react";
import { FaBars, FaSun, FaTimes } from "react-icons/fa";
import Image from "next/image"; // Import the Image component
import { FaMoon } from "react-icons/fa6";

export default function Navbar({ darkMode, setDarkMode }: { darkMode: boolean, setDarkMode: (darkMode: boolean) => void }) {
  const [nav, setNav] = useState(false);
  const [learnOpen, setLearnOpen] = useState(false);
  const [buildOpen, setBuildOpen] = useState(false);
  const [connectOpen, setConnectOpen] = useState(false);


  // Load dark mode preference from localStorage on component mount
  useEffect(() => {
    const storedDarkMode = localStorage.getItem('darkMode');
    if (storedDarkMode === 'true') {
      setDarkMode(true);
    }
  }, []); // Empty dependency array ensures this runs only once on mount

  // Save dark mode preference to localStorage whenever it changes
  useEffect(() => {
    localStorage.setItem('darkMode', darkMode.toString());
  }, [darkMode]);

  return (
    <nav className={`m-2 backdrop-blur-md bg-opacity-50 flex justify-between items-center w-[calc(100%-40px)] z-20 h-20 mx-auto px-6 py-4 fixed top-0 left-0 right-0 rounded-lg transition-all duration-300 ${darkMode ? 'bg-gray-900 text-white' : 'bg-white text-black'}`}>
      <div className="flex items-center"> {/* Wrap logo and text in a flex container */}
        <Image
          src="/kariicon1.png" // Replace with the path to your logo image
          alt="Kanari Logo"
          width={42} // Adjust width as needed
          height={42} // Adjust height as needed
        />
        <h1 className="text-3xl font-signature ml-2">
          <Link
            className={`link-underline ${darkMode ? 'text-orange-200 hover:text-white' : 'text-orange-500 hover:text-black'} hover:scale-105 duration-200 link-underline`}
            href="/"
          >
            Kanari
          </Link>
        </h1>
      </div>

      <ul className="hidden md:flex justify-center items-center">
        {/* Add "Home" link */}
        <li className={`nav-links px-4 cursor-pointer capitalize font-medium relative group ${darkMode ? 'text-orange-200 hover:text-white' : 'text-orange-500 hover:text-black'} hover:scale-105 duration-200 link-underline`}>
          <Link href="/">Home</Link>
        </li>

        {/* Repeat the same structure for "Learn", "Build", and "Connect" */}
        <li className="nav-item relative group"
          onMouseEnter={() => setLearnOpen(true)}
          onMouseLeave={() => setLearnOpen(false)}
        >
          <Link
            href=""
            className={`nav-links px-4 cursor-pointer capitalize font-medium relative group ${darkMode ? 'text-orange-200 hover:text-white' : 'text-orange-500 hover:text-black'} hover:scale-105 duration-200 link-underline`}
            onClick={(e) => {
              e.preventDefault();
              // Toggle dropdown on click for desktop
              setLearnOpen(!learnOpen);
            }}
          >
            Learn
            {/* Optional: Add a dropdown icon */}
            <svg className={`inline-block ml-2 transform transition duration-200 ${learnOpen ? 'rotate-180' : ''}`} width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M6 9L10.2929 4.70711C10.6834 4.31658 10.6834 3.68342 10.2929 3.29289L9.70711 2.70711C9.31658 2.31658 8.68342 2.31658 8.29289 2.70711L6 5.00001L3.70711 2.70711C3.31658 2.31658 2.68342 2.31658 2.29289 2.70711L1.70711 3.29289C1.31658 3.68342 1.31658 4.31658 1.70711 4.70711L6 9Z" fill="currentColor" />
            </svg>
          </Link>
          <div className={`absolute bottom-0 left-0 w-full h-1 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left ${darkMode ? 'bg-gradient-to-r from-orange-400 to-yellow-300' : 'bg-gradient-to-r from-orange-500 to-yellow-500'}`}></div>
          <ul className={`absolute top-full left-0 shadow-lg rounded-lg w-128 py-2 px-4 opacity-0 group-hover:opacity-100 transition-opacity duration-300 invisible group-hover:visible transform z-10 ${darkMode ? 'bg-gray-800' : 'bg-white'}`}>
            <div className="grid grid-cols-3 gap-4">
              <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
                <Link href="/learn/basics" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Basics</Link>
              </li>
              <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
                <Link href="/learn/advanced" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Advanced</Link>
              </li>
              <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
                <Link href="/learn/tutorials" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Tutorials</Link>
              </li>
            </div>
            <div className="grid grid-cols-3 gap-4">
              <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
                <Link href="/learn/basics" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Basics</Link>
              </li>
              <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
                <Link href="/learn/advanced" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Advanced</Link>
              </li>
            </div>
          </ul>
        </li>

        <li className="nav-item relative group"
          onMouseEnter={() => setBuildOpen(true)}
          onMouseLeave={() => setBuildOpen(false)}
        >
          <Link
            href=""
            className={`nav-links px-4 cursor-pointer capitalize font-medium relative group ${darkMode ? 'text-orange-200 hover:text-white' : 'text-orange-500 hover:text-black'} hover:scale-105 duration-200 link-underline`}
            onClick={(e) => {
              e.preventDefault();
              // Toggle dropdown on click for desktop
              setBuildOpen(!buildOpen);
            }}
          >
            Build
            {/* Optional: Add a dropdown icon */}
            <svg className={`inline-block ml-2 transform transition duration-200 ${buildOpen ? 'rotate-180' : ''}`} width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M6 9L10.2929 4.70711C10.6834 4.31658 10.6834 3.68342 10.2929 3.29289L9.70711 2.70711C9.31658 2.31658 8.68342 2.31658 8.29289 2.70711L6 5.00001L3.70711 2.70711C3.31658 2.31658 2.68342 2.31658 2.29289 2.70711L1.70711 3.29289C1.31658 3.68342 1.31658 4.31658 1.70711 4.70711L6 9Z" fill="currentColor" />
            </svg>
          </Link>
          <div className={`absolute bottom-0 left-0 w-full h-1 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left ${darkMode ? 'bg-gradient-to-r from-orange-400 to-yellow-300' : 'bg-gradient-to-r from-orange-500 to-yellow-500'}`}></div>
          <ul className={`absolute top-full left-0 shadow-lg rounded-lg w-128 py-2 px-4 opacity-0 group-hover:opacity-100 transition-opacity duration-300 invisible group-hover:visible transform z-10 ${darkMode ? 'bg-gray-800' : 'bg-white'}`}>
            <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
              <Link href="/build/projects" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Projects</Link>
            </li>
            <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
              <Link href="/build/tools" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Tools</Link>
            </li>
            <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
              <Link href="/build/resources" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Resources</Link>
            </li>
          </ul>
        </li>


        <li className="nav-item relative group"
          onMouseEnter={() => setConnectOpen(true)}
          onMouseLeave={() => setConnectOpen(false)}
        >
          <Link
            href=""
            className={`nav-links px-4 cursor-pointer capitalize font-medium ${darkMode ? 'text-orange-200 hover:text-white' : 'text-orange-500 hover:text-black'} hover:scale-105 duration-200 link-underline`}
            onClick={(e) => {
              e.preventDefault();
              // Toggle dropdown on click for desktop
              setConnectOpen(!connectOpen);
            }}
          >
            Connect
            {/* Optional: Add a dropdown icon */}
            <svg className={`inline-block ml-2 transform transition duration-200 ${connectOpen ? 'rotate-180' : ''}`} width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M6 9L10.2929 4.70711C10.6834 4.31658 10.6834 3.68342 10.2929 3.29289L9.70711 2.70711C9.31658 2.31658 8.68342 2.31658 8.29289 2.70711L6 5.00001L3.70711 2.70711C3.31658 2.31658 2.68342 2.31658 2.29289 2.70711L1.70711 3.29289C1.31658 3.68342 1.31658 4.31658 1.70711 4.70711L6 9Z" fill="currentColor" />
            </svg>
          </Link>
          <div className={`absolute bottom-0 left-0 w-full h-1 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left ${darkMode ? 'bg-gradient-to-r from-orange-400 to-yellow-300' : 'bg-gradient-to-r from-orange-500 to-yellow-500'}`}></div>
          <ul className={`absolute top-full left-0 shadow-lg rounded-lg w-128 py-2 px-4 opacity-0 group-hover:opacity-100 transition-opacity duration-300 invisible group-hover:visible transform z-10 ${darkMode ? 'bg-gray-800' : 'bg-white'}`}>
            <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
              <Link href="/connect/community" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Community</Link>
            </li>
            <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
              <Link href="/connect/events" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Events</Link>
            </li>
            <li className={`py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 ${darkMode ? 'hover:bg-gray-600' : 'hover:bg-gray-100'}`}>
              <Link href="/connect/contact" className={`block px-2 py-1 ${darkMode ? 'text-white' : 'text-gray-800'}`}>Contact Us</Link>
            </li>
          </ul>
        </li>


      </ul>

      {/* Keep Dark Mode Toggle separate */}
      <ul className="hidden md:flex">
        <li className="nav-links px-4 cursor-pointer capitalize font-medium">
          <button onClick={() => setDarkMode(!darkMode)}>
            {darkMode ? <FaSun size={20} /> : <FaMoon size={20} />}
          </button>
        </li>
      </ul>

      <div
        onClick={() => setNav(!nav)}
        className={`cursor-pointer pr-4 z-10 ${darkMode ? 'text-white' : 'text-white'} md:hidden`}
      >
        {nav ? <FaTimes size={30} /> : <FaBars size={30} />}
      </div>

      {nav && (
        <ul className={`flex flex-col justify-center items-center absolute top-0 left-0 w-full h-160 rounded-lg ${darkMode ? 'bg-gray-900 text-white shadow-2xl transform transition duration-500 ease-in-out translate-x-0 opacity-100' : 'bg-gradient-to-r from-orange-500 to-yellow-500 text-orange-200 shadow-2xl transform transition duration-500 ease-in-out -translate-x-full opacity-0'} ${nav ? 'translate-x-0 opacity-100' : '-translate-x-full opacity-0'}`}>
          <li className="px-4 cursor-pointer capitalize py-6 text-4xl transform transition duration-300 ease-in-out hover:scale-110">
            <Link onClick={() => setNav(!nav)} href="/">
              Home
            </Link>
          </li>

          <li className="nav-item relative group px-4 py-6">
            <Link
              href="#"
              className="capitalize text-4xl transform transition duration-300 ease-in-out hover:scale-110"
              onClick={(e) => {
                e.preventDefault(); // Prevent default link behavior
                setLearnOpen(!learnOpen); // Toggle dropdown
              }}
            >
              Learn
              {/* Optional: Add a dropdown icon */}
              <svg className={`inline-block ml-2 transform transition duration-200 ${learnOpen ? 'rotate-180' : ''}`} width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M6 9L10.2929 4.70711C10.6834 4.31658 10.6834 3.68342 10.2929 3.29289L9.70711 2.70711C9.31658 2.31658 8.68342 2.31658 8.29289 2.70711L6 5.00001L3.70711 2.70711C3.31658 2.31658 2.68342 2.31658 2.29289 2.70711L1.70711 3.29289C1.31658 3.68342 1.31658 4.31658 1.70711 4.70711L6 9Z" fill="currentColor" />
              </svg>
            </Link>
            {learnOpen && (
              <ul className={`absolute top-full left-0 bg-white dark:bg-gray-800 shadow-lg rounded-md py-2 px-4 w-48 transform transition duration-300 ease-in-out opacity-0 ${learnOpen ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-2'}`}>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/learn/basics" className="block px-2 py-1 text-gray-800 dark:text-white">Basics</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/learn/advanced" className="block px-2 py-1 text-gray-800 dark:text-white">Advanced</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/learn/tutorials" className="block px-2 py-1 text-gray-800 dark:text-white">Tutorials</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/learn/basics" className="block px-2 py-1 text-gray-800 dark:text-white">Basics</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/learn/advanced" className="block px-2 py-1 text-gray-800 dark:text-white">Advanced</Link>
                </li>
              </ul>
            )}
          </li>

          <li className="nav-item relative group px-4 py-6">
            <Link
              href="#"
              className="capitalize text-4xl transform transition duration-300 ease-in-out hover:scale-110"
              onClick={(e) => {
                e.preventDefault();
                setBuildOpen(!buildOpen);
              }}
            >
              Build
              {/* Optional: Add a dropdown icon */}
              <svg className={`inline-block ml-2 transform transition duration-200 ${buildOpen ? 'rotate-180' : ''}`} width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M6 9L10.2929 4.70711C10.6834 4.31658 10.6834 3.68342 10.2929 3.29289L9.70711 2.70711C9.31658 2.31658 8.68342 2.31658 8.29289 2.70711L6 5.00001L3.70711 2.70711C3.31658 2.31658 2.68342 2.31658 2.29289 2.70711L1.70711 3.29289C1.31658 3.68342 1.31658 4.31658 1.70711 4.70711L6 9Z" fill="currentColor" />
              </svg>
            </Link>
            {buildOpen && (
              <ul className={`absolute top-full left-0 bg-white dark:bg-gray-800 shadow-lg rounded-md py-2 px-4 w-48 transform transition duration-300 ease-in-out opacity-0 ${buildOpen ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-2'}`}>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/build/projects" className="block px-2 py-1 text-gray-800 dark:text-white">Projects</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/build/tools" className="block px-2 py-1 text-gray-800 dark:text-white">Tools</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/build/resources" className="block px-2 py-1 text-gray-800 dark:text-white">Resources</Link>
                </li>
              </ul>
            )}
          </li>

          <li className="nav-item relative group px-4 py-6">
            <Link
              href="#"
              className="capitalize text-4xl transform transition duration-300 ease-in-out hover:scale-110"
              onClick={(e) => {
                e.preventDefault();
                setConnectOpen(!connectOpen);
              }}
            >
              Connect
              {/* Optional: Add a dropdown icon */}
              <svg className={`inline-block ml-2 transform transition duration-200 ${connectOpen ? 'rotate-180' : ''}`} width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M6 9L10.2929 4.70711C10.6834 4.31658 10.6834 3.68342 10.2929 3.29289L9.70711 2.70711C9.31658 2.31658 8.68342 2.31658 8.29289 2.70711L6 5.00001L3.70711 2.70711C3.31658 2.31658 2.68342 2.31658 2.29289 2.70711L1.70711 3.29289C1.31658 3.68342 1.31658 4.31658 1.70711 4.70711L6 9Z" fill="currentColor" />
              </svg>
            </Link>
            {connectOpen && (
              <ul className={`absolute top-full left-0 bg-white dark:bg-gray-800 shadow-lg rounded-md py-2 px-4 w-48 transform transition duration-300 ease-in-out opacity-0 ${connectOpen ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-2'}`}>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/connect/community" className="block px-2 py-1 text-gray-800 dark:text-white">Community</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/connect/events" className="block px-2 py-1 text-gray-800 dark:text-white">Events</Link>
                </li>
                <li className="py-1 rounded-md transition-colors duration-200 ease-in-out transform hover:-translate-y-1 hover:bg-gray-100 dark:hover:bg-gray-700">
                  <Link href="/connect/contact" className="block px-2 py-1 text-gray-800 dark:text-white">Contact Us</Link>
                </li>
              </ul>
            )}
          </li>

          <li className="nav-links px-4 cursor-pointer capitalize font-medium py-6 transform transition duration-300 ease-in-out hover:scale-110">
            {/* Dark Mode Toggle */}
            <button onClick={() => setDarkMode(!darkMode)}>
              {darkMode ? <FaSun size={20} /> : <FaMoon size={20} />}
            </button>
          </li>

        </ul>
      )}
    </nav>
  );
}