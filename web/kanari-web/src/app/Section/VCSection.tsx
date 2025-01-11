"use client";

import { useState } from 'react';

export function VCSection() {
    // Sample VC and Investor data (replace with your actual data)
    const vcs = [
        {
            name: "jamesatomc",
            logo: "/jamesatomc.png",
            investmentDetails: "Invested $1 million in Series A funding.",
        },
        // ... add more VCs with investment details
    ];

    const [selectedVC, setSelectedVC] = useState<number | null>(null);

    return (
        <>
            {/* VC Section */}
            <section className="py-20 px-4 bg-gray-50 dark:bg-gray-900 relative">
                <div className="max-w-7xl mx-auto text-center">
                    <h2 className="text-3xl md:text-4xl font-bold text-gray-800 dark:text-white mb-6">
                        Backed by Leading Investors
                    </h2>
                    <p className="text-gray-600 dark:text-gray-300 mb-10">
                        Kanari Network is supported by a strong network of investors who believe in our vision.
                    </p>
                    <div className="flex flex-wrap justify-center">
                        {vcs.map((vc, index) => (
                            <button
                                key={index}
                                onClick={() => setSelectedVC(index)}
                                className="relative w-40 h-48 m-4 rounded-lg overflow-hidden shadow-lg 
                                           transform transition duration-300 ease-in-out hover:-translate-y-2 hover:shadow-xl"
                            >
                                <img
                                    src={vc.logo}
                                    alt={vc.name}
                                    className="object-cover w-full h-full"
                                />
                                <div className="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent opacity-0 hover:opacity-100 transition duration-300"></div>
                                <span className="absolute bottom-4 left-4 text-orange-200 dark:text-white font-bold text-lg">
                                    {vc.name}
                                </span>
                            </button>
                        ))}
                    </div>
                </div>
            </section>

            {/* Modal/Popup for VC Details */}
            {selectedVC !== null && (
                <div
                    className="fixed inset-0 flex items-center justify-center z-50 bg-black/50 backdrop-blur-sm"
                    onClick={() => setSelectedVC(null)}
                >
                    <div
                        className="bg-white dark:bg-gray-800 w-11/12 md:w-2/3 lg:w-1/2 p-6 md:p-10 rounded-lg shadow-2xl relative transform transition-transform duration-300 ease-in-out"
                        onClick={(e) => e.stopPropagation()}
                    >
                        <button
                            className="absolute top-4 right-4 text-gray-500 hover:text-gray-700"
                            onClick={() => setSelectedVC(null)}
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                className="h-6 w-6"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke="currentColor"
                            >
                                <path
                                    strokeLinecap="round"
                                    strokeLinejoin="round"
                                    strokeWidth={2}
                                    d="M6 18L18 6M6 6l12 12"
                                />
                            </svg>
                        </button>
                        <div className="flex flex-col items-center">
                            <img
                                src={vcs[selectedVC].logo}
                                alt={vcs[selectedVC].name}
                                className="w-32 h-32 rounded-full mb-6"
                            />
                            <h3 className="text-2xl font-bold text-gray-800 dark:text-white mb-4">
                                {vcs[selectedVC].name}
                            </h3>
                            <p className="text-gray-600 dark:text-gray-300 text-center">
                                {vcs[selectedVC].investmentDetails}
                            </p>
                        </div>
                    </div>
                </div>
            )}

        </>
    );
}