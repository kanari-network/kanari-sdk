'use client';

import Image from 'next/image';
import Link from 'next/link';
import { ReactNode, useEffect, useRef, useState } from 'react';

export function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 20 20" className="h-4 w-4" fill="none">
      <path d="M3 10h13m-5-5 5 5-5 5" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" />
    </svg>
  );
}

export function SiteHeader() {
  const [menuOpen, setMenuOpen] = useState(false);
  const [darkMode, setDarkMode] = useState(false);
  const [headerVisible, setHeaderVisible] = useState(true);
  const previousScrollY = useRef(0);

  useEffect(() => {
    const syncTheme = window.setTimeout(() => {
      setDarkMode(document.documentElement.classList.contains('dark'));
    }, 0);

    return () => window.clearTimeout(syncTheme);
  }, []);

  useEffect(() => {
    const onScroll = () => {
      const currentScrollY = window.scrollY;
      const scrollingUp = currentScrollY < previousScrollY.current;

      setHeaderVisible(menuOpen || currentScrollY < 80 || scrollingUp);
      previousScrollY.current = currentScrollY;
    };

    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  }, [menuOpen]);

  const toggleTheme = () => {
    const nextDarkMode = !darkMode;
    document.documentElement.classList.toggle('dark', nextDarkMode);
    localStorage.setItem('theme', nextDarkMode ? 'dark' : 'light');
    setDarkMode(nextDarkMode);
  };

  return (
    <>
      <header className={`site-header ${headerVisible ? '' : 'site-header--hidden'}`}>
        <Link href="/" className="brand" aria-label="Kanari Network home">
          <Image src="/kariicon1.png" alt="" width={42} height={42} priority />
          <span>KANARI</span>
        </Link>

        <nav className={`site-nav ${menuOpen ? 'site-nav--open' : ''}`} aria-label="Main navigation">
          <Link href="/" onClick={() => setMenuOpen(false)}>Overview</Link>
          <Link href="/tx" onClick={() => setMenuOpen(false)}>Transactions</Link>
          <Link href="/coins" onClick={() => setMenuOpen(false)}>Tokens</Link>
          <Link href="/modules" onClick={() => setMenuOpen(false)}>Modules</Link>
          <Link href="/account" onClick={() => setMenuOpen(false)}>Accounts</Link>
          <Link href="/nft" onClick={() => setMenuOpen(false)}>NFTs</Link>
        </nav>
        <div className="site-header__actions">
          <button className="theme-toggle" type="button" onClick={toggleTheme} aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}>
            {darkMode ? (
              <svg aria-hidden="true" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
            ) : (
              <svg aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M20.5 14.4A8 8 0 0 1 9.6 3.5 8.5 8.5 0 1 0 20.5 14.4Z" /></svg>
            )}
          </button>
          <button
            className="menu-toggle"
            type="button"
            aria-label="Toggle navigation"
            aria-expanded={menuOpen}
            onClick={() => setMenuOpen(!menuOpen)}
          >
            <span />
            <span />
          </button>
          <a className="header-button" href="https://docs.kanarinetwork.site/" target="_blank" rel="noreferrer">
            Read the docs <ArrowIcon />
          </a>
        </div>
      </header>
      <div className="site-header-space" aria-hidden="true" />
    </>
  );
}

export function SiteFooter() {
  return (
    <footer className="site-footer section-wrap">
      <Link href="/" className="brand">
        <Image src="/kariicon1.png" alt="" width={36} height={36} />
        <span>KANARI</span>
      </Link>
      <p>Community-powered infrastructure for digital ownership.</p>
      <div>
        <Link href="https://kanarinetwork.site//DeveloperPortal">Developers</Link>
        <Link href="https://kanarinetwork.site//connect/ecosystem">Ecosystem</Link>
        <Link href="https://kanarinetwork.site//connect/community">Community</Link>
        <Link href="https://kanarinetwork.site//KanariFoundation">Foundation</Link>
        <Link href="https://kanarinetwork.site/MediaKit">Media kit</Link>
        <Link href="https://kanarinetwork.site//Team">Team</Link>
        <a href="https://github.com/kanari-network" target="_blank" rel="noreferrer">GitHub</a>
        <a href="https://docs.kanarinetwork.site/" target="_blank" rel="noreferrer">Docs</a>
        <Link href="https://kanarinetwork.site//PrivacyPolicy">Privacy</Link>
      </div>
    </footer>
  );
}

export function PageShell({ children }: { children: ReactNode }) {
  return (
    <main className="site-shell">
      <div className="site-noise" />
      <SiteHeader />
      {children}
      <SiteFooter />
    </main>
  );
}

interface PageHeroProps {
  kicker: string;
  title: string;
  accent: string;
  description: string;
  children?: ReactNode;
}

export function PageHero({ kicker, title, accent, description, children }: PageHeroProps) {
  return (
    <section className="subpage-hero section-wrap">
      <p className="section-kicker">{kicker}</p>
      <h1>{title}<br /><span>{accent}</span></h1>
      <p className="subpage-hero__description">{description}</p>
      {children && <div className="hero-actions">{children}</div>}
    </section>
  );
}

export function SectionHeading({ kicker, title, description }: { kicker: string; title: string; description?: string }) {
  return (
    <div className="subpage-section-heading">
      <p className="section-kicker">{kicker}</p>
      <h2>{title}</h2>
      {description && <p>{description}</p>}
    </div>
  );
}

export function PageCTA({ kicker, title, description, href, label }: { kicker: string; title: string; description: string; href: string; label: string }) {
  const opensNewTab = href.startsWith('mailto:') || href.startsWith('http');

  return (
    <section className="subpage-cta section-wrap">
      <p className="section-kicker">{kicker}</p>
      <h2>{title}</h2>
      <p>{description}</p>
      <a className="button button--dark" href={href} target={opensNewTab ? '_blank' : undefined} rel={opensNewTab ? 'noreferrer' : undefined}>{label} <ArrowIcon /></a>
    </section>
  );
}
