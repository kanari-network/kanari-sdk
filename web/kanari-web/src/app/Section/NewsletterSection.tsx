"use client";

import { useState } from 'react';

export function NewsletterSection() {

    return (
        <>
            <section className="px-4 py-20 bg-gradient-to-r from-orange-500 to-yellow-500 dark:from-gray-800 dark:to-gray-900">
                <div className="max-w-4xl mx-auto rounded-lg shadow-lg p-6 sm:p-8 bg-white/10 backdrop-blur-md dark:bg-gray-800/20">
                    <h2 className="text-3xl sm:text-4xl font-bold text-white text-center mb-4">
                        Subscribe to Kanari Network Newsletter
                    </h2>
                    <p className="text-base sm:text-lg text-white text-center mb-6">
                        Get the latest news, updates, and insights delivered straight to your
                        inbox.
                    </p>
                    <form className="flex flex-col items-center space-y-4 w-full">
                        <input
                            type="email"
                            id="Email"
                            className="w-full px-4 py-2 sm:py-3 text-sm text-gray-900 bg-white/20 border border-gray-400 rounded-md focus:ring-blue-500 focus:border-blue-500 placeholder:text-blue-50 dark:bg-gray-700/20 dark:border-gray-600 dark:placeholder:text-gray-400"
                            placeholder="Enter your email"
                            required
                        />
                        <button
                            type="submit"
                            className="bg-white hover:bg-gray-100 text-black font-medium py-2 px-6 rounded-full transition duration-300 ease-in-out transform hover:scale-105 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-white"
                        >
                            Subscribe
                        </button>
                    </form>
                    <div className="flex items-center justify-center mt-4 text-white">
                        <input
                            id="link-checkbox"
                            type="checkbox"
                            value=""
                            className="w-4 h-4 text-black border-gray-400 rounded focus:ring-blue-500 dark:border-gray-600"
                        />
                        <label
                            htmlFor="link-checkbox"
                            className="ml-2 text-xs sm:text-sm"
                        >
                            I agree with the
                            <a
                                href="#"
                                className="text-blue-300 hover:underline"
                            >
                                terms and conditions
                            </a>
                            .
                        </label>
                    </div>
                </div>
            </section>

        </>
    );
}



