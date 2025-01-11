'use client'; // Add this line

import { useState } from 'react';
import Image from 'next/image';

interface FooterProps {
  darkMode: boolean;
  setDarkMode: (darkMode: boolean) => void;
}

const Footer: React.FC<FooterProps> = ({ darkMode, setDarkMode }) => {
  const [discordHover, setDiscordHover] = useState(false);
  const [twitterHover, setTwitterHover] = useState(false);
  const [githubHover, setGithubHover] = useState(false);

  return (
    <div className="pt-4 px-4 pb-4">
      <footer className={`rounded-lg shadow-lg p-6 sm:p-8 backdrop-blur-md  ${darkMode ? ' text-black bg-white' : 'bg-white/10 text-white'}`}>         
      <div className="mx-auto w-full max-w-screen-xl p-4 py-6 lg:py-8">
          <div className="md:flex md:justify-between">
            {/* Logo */}
            <div className="mb-6 md:mb-0">
              <a href="#" className="flex items-center">
                <Image
                  src="/kariicon1.png"
                  className="h-8 mr-3"
                  alt="KanariLogo"
                  width={32}
                  height={32}
                />
                <span className="self-center text-2xl font-semibold whitespace-nowrap">
                  Kanari Network
                </span>
              </a>
            </div>

            {/* Links */}
            <div className="grid grid-cols-2 gap-8 sm:gap-6 sm:grid-cols-3">
              {/* Resources */}
              <div>
                <h2 className="mb-6 text-sm font-semibold">
                  Resources
                </h2>
                <ul className="font-medium">
                  <li className="mb-4">
                    <a href="#" className="hover:underline">
                      Developer portal
                    </a>
                  </li>
                  <li className="mb-4">
                    <a href="https://docs.kanari.network/" className="hover:underline">
                      Docs
                    </a>
                  </li>
                  <li className="mb-4">
                    <a href="#" className="hover:underline">
                      Whitepaper
                    </a>
                  </li>
{/*                   <li className="mb-4">
                    <a href="https://kanarinet.slack.com/" className="hover:underline">
                      Developer forum
                    </a>
                  </li> */}
                </ul>
              </div>

              {/* Solutions */}
              <div>
                <h2 className="mb-6 text-sm font-semibold">
                  Solutions
                </h2>
                <ul className="font-medium">
                  <li className="mb-4">
                    <a href="https://github.com/jamesatomc/walletkari" className="hover:underline">
                      Kanari Pay soon
                    </a>
                  </li>
                  <li className="mb-4">
                    <a href="#" className="hover:underline">
                      AI soon
                    </a>
                  </li>
                  <li className="mb-4">
                    <a href="https://github.com/kanari-network/kanari-network" className="hover:underline">
                      Blockchain
                    </a>
                  </li>
                </ul>
              </div>

              {/* About */}
              <div>
                <h2 className="mb-6 text-sm font-semibold">
                  About
                </h2>
                <ul className="font-medium">
                  <li className="mb-4">
                    <a href="#" className="hover:underline">
                      Kanari Foundation
                    </a>
                  </li>
                  <li className="mb-4">
                    <a href="https://drive.google.com/drive/folders/1cWm3pSEKFew0zhHU6OzTNzwSfnS5n4MR" className="hover:underline">
                      Media kit
                    </a>
                  </li>
                  <li className="mb-4">
                    <a href="#" className="hover:underline">
                      Team
                    </a>
                  </li>
                  <li>
                    <a href="https://blog.kanari.network/" className="hover:underline">
                      Blog
                    </a>
                  </li>
                </ul>
              </div>
            </div>
          </div>

          {/* Copyright and Social */}
          <hr className="my-6 border-gray-700 sm:mx-auto lg:my-8" />
          <div className="sm:flex sm:items-center sm:justify-between">
            <span className="text-sm sm:text-center">
              © 2024{' '}
              <a href="#" className="hover:underline">
                Kanari Network™
              </a>
              . community.
            </span>
            <div className="flex mt-4 sm:justify-center sm:mt-0 space-x-6">
              {/* Discord */}
              <a
                href="https://discord.gg/QU6fTacXab"
                className="transform transition duration-300 ease-in-out"
                onMouseEnter={() => setDiscordHover(true)}
                onMouseLeave={() => setDiscordHover(false)}
              >
                <svg
                  className={`w-5 h-5 ${discordHover ? 'transform scale-125' : ''}`}
                  aria-hidden="true"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="currentColor"
                  viewBox="0 0 21 16"
                >
                  {/* Discord icon path */}
                  <path d="M16.942 1.556a16.3 16.3 0 0 0-4.126-1.3 12.04 12.04 0 0 0-.529 1.1 15.175 15.175 0 0 0-4.573 0 11.585 11.585 0 0 0-.535-1.1 16.274 16.274 0 0 0-4.129 1.3A17.392 17.392 0 0 0 .182 13.218a15.785 15.785 0 0 0 4.963 2.521c.41-.564.773-1.16 1.084-1.785a10.63 10.63 0 0 1-1.706-.83c.143-.106.283-.217.418-.33a11.664 11.664 0 0 0 10.118 0c.137.113.277.224.418.33-.544.328-1.116.606-1.71.832a12.52 12.52 0 0 0 1.084 1.785 16.46 16.46 0 0 0 5.064-2.595 17.286 17.286 0 0 0-2.973-11.59ZM6.678 10.813a1.941 1.941 0 0 1-1.8-2.045 1.93 1.93 0 0 1 1.8-2.047 1.919 1.919 0 0 1 1.8 2.047 1.93 1.93 0 0 1-1.8 2.045Zm6.644 0a1.94 1.94 0 0 1-1.8-2.045 1.93 1.93 0 0 1 1.8-2.047 1.918 1.918 0 0 1 1.8 2.047 1.93 1.93 0 0 1-1.8 2.045Z" />
                </svg>
                <span className="sr-only">Discord community</span>
              </a>

              {/* Twitter */}
              <a
                href="https://x.com/kanari_netwok"
                className="transform transition duration-300 ease-in-out"
                onMouseEnter={() => setTwitterHover(true)}
                onMouseLeave={() => setTwitterHover(false)}
              >
                <svg
                  className={`w-5 h-5 ${twitterHover ? 'transform scale-125' : ''}`}
                  aria-hidden="true"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="currentColor"
                  viewBox="0 0 20 17"
                >
                  {/* Twitter icon path */}
                  <path
                    fillRule="evenodd"
                    d="M20 1.892a8.178 8.178 0 0 1-2.355.635 4.074 4.074 0 0 0 1.8-2.235 8.344 8.344 0 0 1-2.605.98A4.13 4.13 0 0 0 13.85 0a4.068 4.068 0 0 0-4.1 4.038 4 4 0 0 0 .105.919A11.705 11.705 0 0 1 1.4.734a4.006 4.006 0 0 0 1.268 5.392 4.165 4.165 0 0 1-1.859-.5v.05A4.057 4.057 0 0 0 4.1 9.635a4.19 4.19 0 0 1-1.856.07 4.108 4.108 0 0 0 3.831 2.807A8.36 8.36 0 0 1 0 14.184 11.732 11.732 0 0 0 6.291 16 11.502 11.502 0 0 0 17.964 4.5c0-.177 0-.35-.012-.523A8.143 8.143 0 0 0 20 1.892Z"
                    clipRule="evenodd"
                  />
                </svg>
                <span className="sr-only">Twitter page</span>
              </a>

              {/* Github */}
              <a
                href="https://github.com/kanari-network"
                className="transform transition duration-300 ease-in-out"
                onMouseEnter={() => setGithubHover(true)}
                onMouseLeave={() => setGithubHover(false)}
              >
                <svg
                  className={`w-5 h-5 ${githubHover ? 'transform scale-125' : ''}`}
                  aria-hidden="true"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                >
                  {/* Github icon path */}
                  <path
                    fillRule="evenodd"
                    d="M10 .333A9.911 9.911 0 0 0 6.866 19.65c.5.092.678-.215.678-.477 0-.237-.01-1.017-.014-1.845-2.757.6-3.338-1.169-3.338-1.169a2.627 2.627 0 0 0-1.1-1.451c-.9-.615.07-.6.07-.6a2.084 2.084 0 0 1 1.518 1.021 2.11 2.11 0 0 0 2.884.823c.044-.503.268-.973.63-1.325-2.2-.25-4.516-1.1-4.516-4.9A3.832 3.832 0 0 1 4.7 7.068a3.56 3.56 0 0 1 .095-2.623s.832-.266 2.726 1.016a9.409 9.409 0 0 1 4.962 0c1.89-1.282 2.717-1.016 2.717-1.016.366.83.402 1.768.1 2.623a3.827 3.827 0 0 1 1.02 2.659c0 3.807-2.319 4.644-4.525 4.889a2.366 2.366 0 0 1 .673 1.834c0 1.326-.012 2.394-.012 2.72 0 .263.18.572.681.475A9.911 9.911 0 0 0 10 .333Z"
                    clipRule="evenodd"
                  />
                </svg>
                <span className="sr-only">GitHub account</span>
              </a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
};

export default Footer;
