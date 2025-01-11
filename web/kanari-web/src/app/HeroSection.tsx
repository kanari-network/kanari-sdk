'use client'; // In HeroSection.tsx
import { NextPage } from 'next'
import { VCSection } from './Section/VCSection'
import { NewsletterSection } from './Section/NewsletterSection'
import Navbar from './Section/body/Navbar'
import Footer from './Section/body/Footer'
import { useState } from 'react'




interface Props { }


const HeroSection: NextPage<Props> = ({ }) => {
    const [darkMode, setDarkMode] = useState(false);


    return (

        <div className={darkMode ? 'dark' : 'bg-gradient-to-r from-orange-500 to-yellow-500'}>

            <Navbar darkMode={darkMode} setDarkMode={setDarkMode} />
        
            <main className="relative">
                {/* Hero Section */}
                <section className="relative  overflow-hidden h-190">
                    <div className="parallax-bg"> {/* Add parallax class */}
                        <video
                            autoPlay
                            loop
                            muted
                            className="object-cover h-[700px] w-full"
                        >
                            <source
                                src="/Lines.mp4"
                                type="video/mp4"
                            />
                        </video>
                    </div>
                    <div className="absolute top-0 left-0 w-full h-full flex items-center justify-center bg-gradient-to-r from-orange-600 to-yellow-500 dark:from-gray-800 dark:to-gray-900 opacity-80">
                        <div className="max-w-4xl text-center space-y-6">
                            <h1 className="text-white text-5xl sm:text-6xl md:text-7xl lg:text-8xl font-bold leading-tight">
                                Build Smart Contracts with Ease
                            </h1>
                            <p className="text-white text-lg sm:text-xl md:text-2xl font-medium">
                                Kanari Network empowers developers to create secure,
                                transparent, and efficient blockchain applications.
                            </p>
                            <div className="flex justify-center space-x-4">
                                <a
                                    href="#"
                                    className="bg-orange-500 hover:bg-orange-700 text-white px-6 py-3 rounded-full font-medium transition duration-300 ease-in-out transform hover:scale-105"
                                >
                                    Start Building
                                </a>
                                <a
                                    href="#"
                                    className="bg-white  text-black px-6 py-3 rounded-full font-medium transition duration-300 ease-in-out transform hover:scale-105"
                                >
                                    Learn More
                                </a>
                            </div>
                        </div>
                    </div>
                </section>

                {/* Features Section */}
                <section className="py-20 px-4 bg-gradient-to-r from-gray-100 to-gray-200 dark:from-gray-800 dark:to-gray-900"> {/* Added dark mode gradient */}
                    <div className="max-w-7xl mx-auto text-center">
                        <h2 className="text-4xl font-bold text-gray-800 dark:text-white mb-8"> {/* Added dark:text-white */}
                            Kanari Network: Empowering the Future of Finance
                        </h2>
                        <p className="text-lg text-gray-600 dark:text-gray-300 mb-12"> {/* Added dark:text-gray-300 */}
                            Kanari Network is a decentralized blockchain platform designed to
                            revolutionize the way we interact with financial services. Our
                            cutting-edge technology empowers developers to build secure,
                            transparent, and efficient applications that benefit everyone.
                        </p>
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
                            {/* Feature Card 1 */}
                            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 transform transition duration-300 ease-in-out hover:scale-105 hover:shadow-2xl relative overflow-hidden group perspective-1000"> {/* Added dark:bg-gray-800 */}
                                <div className="relative z-10">
                                    <div className="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900 mb-4 mx-auto shadow-lg transform -rotate-12"> {/* Added dark:bg-blue-900 */}
                                        <svg className="h-6 w-6 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h10a2 2 0 002-2v-2M9 7a2 2 0 012-2h2a2 2 0 012 2m-6 8a2 2 0 002 2h2a2 2 0 002-2" /></svg>
                                    </div>
                                    <h3 className="text-xl font-bold text-gray-800 dark:text-white mb-4"> {/* Added dark:text-white */}
                                        Decentralized Finance
                                    </h3>
                                    <p className="text-gray-600 dark:text-gray-300"> {/* Added dark:text-gray-300 */}
                                        Experience the power of DeFi with Kanari Network. Access a
                                        wide range of financial services without intermediaries,
                                        giving you full control over your assets.
                                    </p>
                                </div>
                                <div className="absolute bottom-0 right-0 w-48 h-48 bg-orange-300 dark:bg-orange-700 transform -rotate-45 translate-x-1/2 group-hover:-translate-x-0 transition duration-500 ease-in-out shadow-2xl rounded-full"></div> {/* Added dark:bg-orange-700 */}
                            </div>

                            {/* Feature Card 2 - Repeat similar structure */}
                            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 transform transition duration-300 ease-in-out hover:scale-105 hover:shadow-2xl relative overflow-hidden group perspective-1000"> {/* Added dark:bg-gray-800 */}
                                <div className="relative z-10">
                                    <div className="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900 mb-4 mx-auto shadow-lg transform rotate-6"> {/* Added dark:bg-blue-900 */}
                                        <svg className="h-6 w-6 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" /></svg>
                                    </div>
                                    <h3 className="text-xl font-bold text-gray-800 dark:text-white mb-4"> {/* Added dark:text-white */}
                                        Smart Contracts
                                    </h3>
                                    <p className="text-gray-600 dark:text-gray-300"> {/* Added dark:text-gray-300 */}
                                        Build and deploy secure and automated smart contracts on Kanari
                                        Network. Our robust infrastructure ensures the integrity and
                                        reliability of your applications.
                                    </p>
                                </div>
                                <div className="absolute bottom-0 right-0 w-48 h-48 bg-orange-300 dark:bg-orange-700 transform -rotate-45 translate-x-1/2 group-hover:-translate-x-0 transition duration-500 ease-in-out shadow-2xl rounded-full"></div> {/* Added dark:bg-orange-700 */}
                            </div>

                            {/* Feature Card 3 - Repeat similar structure */}
                            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 transform transition duration-300 ease-in-out hover:scale-105 hover:shadow-2xl relative overflow-hidden group perspective-1000"> {/* Added dark:bg-gray-800 */}
                                <div className="relative z-10">
                                    <div className="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900 mb-4 mx-auto shadow-lg transform -rotate-6"> {/* Added dark:bg-blue-900 */}
                                        <svg className="h-6 w-6 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2" /><circle cx="9" cy="7" r="4" /><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M23 21v-2a4 4 0 00-3-3.87m-4-12a4 4 0 010 7.75" /></svg>
                                    </div>
                                    <h3 className="text-xl font-bold text-gray-800 dark:text-white mb-4"> {/* Added dark:text-white */}
                                        Community Driven
                                    </h3>
                                    <p className="text-gray-600 dark:text-gray-300"> {/* Added dark:text-gray-300 */}
                                        Join our vibrant community of developers, entrepreneurs, and
                                        enthusiasts. Collaborate, innovate, and shape the future of
                                        Kanari Network together.
                                    </p>
                                </div>
                                <div className="absolute bottom-0 right-0 w-48 h-48 bg-orange-300 dark:bg-orange-700 transform -rotate-45 translate-x-1/2 group-hover:-translate-x-0 transition duration-500 ease-in-out shadow-2xl rounded-full"></div> {/* Added dark:bg-orange-700 */}
                            </div>
                        </div>
                    </div>
                </section>

                {/* Investors Section */}
                <section className="py-20 px-4 bg-gradient-to-r from-white to-gray-100 dark:from-gray-900 dark:to-gray-800"> {/* Added dark mode gradient */}
                    <div className="max-w-7xl mx-auto text-center">
                        <h2 className="text-4xl font-bold text-gray-800 dark:text-white mb-8"> {/* Added dark:text-white */}
                            Kanari Network: Empowering Investors
                        </h2>
                        <p className="text-lg text-gray-600 dark:text-gray-300 mb-12"> {/* Added dark:text-gray-300 */}
                            Kanari Network provides unique opportunities for investors to be
                            part of the decentralized future.
                        </p>
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
                            {/* Investor Card 1 */}
                            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg shadow-lg p-8 transform transition duration-300 ease-in-out hover:scale-105 hover:shadow-2xl relative overflow-hidden group perspective-1000"> {/* Added dark:bg-gray-700 */}
                                <div className="relative z-10"> {/* Wrap content with relative positioning */}
                                    <div className="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900 mb-4 mx-auto shadow-lg transform -rotate-12"> {/* Add shadow and rotate icon, Added dark:bg-blue-900 */}
                                        {/* Icon can be added here */}
                                        <svg className="h-6 w-6 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" /></svg> {/* Added dark:text-blue-400 */}
                                    </div>
                                    <h3 className="text-xl font-bold text-gray-800 dark:text-white mb-4"> {/* Added dark:text-white */}
                                        Early Adoption
                                    </h3>
                                    <p className="text-gray-600 dark:text-gray-300"> {/* Added dark:text-gray-300 */}
                                        Be an early investor in a rapidly growing ecosystem. Kanari
                                        Network is at the forefront of blockchain innovation,
                                        offering substantial growth potential.
                                    </p>
                                </div>
                                {/* Animated Shape */}
                                <div className="absolute bottom-0 right-0 w-48 h-48 bg-orange-300 dark:bg-orange-700 transform -rotate-45 translate-x-1/2 group-hover:-translate-x-0 transition duration-500 ease-in-out shadow-2xl rounded-full"></div> {/* Added dark:bg-orange-700 */}
                            </div>

                            {/* Investor Card 2 - Repeat similar structure */}
                            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg shadow-lg p-8 transform transition duration-300 ease-in-out hover:scale-105 hover:shadow-2xl relative overflow-hidden group perspective-1000"> {/* Added dark:bg-gray-700 */}
                                <div className="relative z-10">
                                    <div className="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900 mb-4 mx-auto shadow-lg transform rotate-6"> {/* Added dark:bg-blue-900 */}
                                        <svg className="h-6 w-6 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 9a2 2 0 10-4 0v12" /><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 9a2 2 0 10-4 0v12" /></svg> {/* Added dark:text-blue-400 */}
                                    </div>
                                    <h3 className="text-xl font-bold text-gray-800 dark:text-white mb-4"> {/* Added dark:text-white */}
                                        Tokenization
                                    </h3>
                                    <p className="text-gray-600 dark:text-gray-300"> {/* Added dark:text-gray-300 */}
                                        Explore new investment avenues with tokenized assets. Kanari
                                        Network facilitates fractional ownership and liquidity,
                                        opening doors to previously inaccessible markets.
                                    </p>
                                </div>
                                <div className="absolute bottom-0 right-0 w-48 h-48 bg-orange-300 dark:bg-orange-700 transform -rotate-45 translate-x-1/2 group-hover:-translate-x-0 transition duration-500 ease-in-out shadow-2xl rounded-full"></div> {/* Added dark:bg-orange-700 */}
                            </div>

                            {/* Investor Card 3 - Repeat similar structure */}
                            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg shadow-lg p-8 transform transition duration-300 ease-in-out hover:scale-105 hover:shadow-2xl relative overflow-hidden group perspective-1000"> {/* Added dark:bg-gray-700 */}
                                <div className="relative z-10">
                                    <div className="flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900 mb-4 mx-auto shadow-lg transform -rotate-6"> {/* Added dark:bg-blue-900 */}
                                        <svg className="h-6 w-6 text-blue-500 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.658 0-3 .895-3 2s1.342 2 3 2 3-.895 3-2-1.342-2-3-2zM3 12c0 4.411 3.589 8 8 8s8-3.589 8-8-3.589-8-8-8-8 3.589-8 8z" /><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4z" /></svg> {/* Added dark:text-blue-400 */}
                                    </div>
                                    <h3 className="text-xl font-bold text-gray-800 dark:text-white mb-4"> {/* Added dark:text-white */}
                                        Decentralized Governance
                                    </h3>
                                    <p className="text-gray-600 dark:text-gray-300"> {/* Added dark:text-gray-300 */}
                                        Participate in the governance of Kanari Network. As a token
                                        holder, you have a voice in shaping the future direction of
                                        the platform.
                                    </p>
                                </div>
                                <div className="absolute bottom-0 right-0 w-48 h-48 bg-orange-300 dark:bg-orange-700 transform -rotate-45 translate-x-1/2 group-hover:-translate-x-0 transition duration-500 ease-in-out shadow-2xl rounded-full"></div> {/* Added dark:bg-orange-700 */}
                            </div>
                        </div>
                    </div>
                </section>


                {/* VC Section */}
{/*                 <VCSection /> */}

                {/* Newsletter Section */}
                <NewsletterSection />


            </main>
            <Footer darkMode={darkMode} setDarkMode={setDarkMode} />

        </div>
    )
}

export default HeroSection
