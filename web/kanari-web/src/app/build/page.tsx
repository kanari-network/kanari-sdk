'use client';
import React, { useState } from 'react';
import Navbar from '../Section/body/Navbar';
import Footer from '../Section/body/Footer';

const Build = () => {
  const [darkMode, setDarkMode] = useState(false);

  return (

    <div className={darkMode ? 'dark' : 'bg-gradient-to-r from-orange-500 to-yellow-500'}>
      <Navbar darkMode={darkMode} setDarkMode={setDarkMode} />
      <section className="py-10">
        {/* Your content for the Learn page */}
      </section>
      <Footer darkMode={darkMode} setDarkMode={setDarkMode} />
    </div>
  );
};

export default Build;