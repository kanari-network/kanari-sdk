'use client';

import Image from 'next/image';
import Link from 'next/link';
import { FormEvent, ReactNode, useEffect, useRef, useState } from 'react';
import { getActiveRpcUrl, RPC_PRESETS, setActiveRpcUrl } from '../lib/rpc';

export function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 20 20" className="h-4 w-4" fill="none">
      <path d="M3 10h13m-5-5 5 5-5 5" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" />
    </svg>
  );
}

export function SiteHeader() {
  const [menuOpen, setMenuOpen] = useState(false);
  const [toolsOpen, setToolsOpen] = useState(false);
  const [darkMode, setDarkMode] = useState(false);
  const [headerVisible, setHeaderVisible] = useState(true);
  const previousScrollY = useRef(0);
  const [rpcDialogOpen, setRpcDialogOpen] = useState(false);
  const [rpcUrl, setRpcUrl] = useState('');
  const [rpcError, setRpcError] = useState('');

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

      setHeaderVisible(menuOpen || toolsOpen || currentScrollY < 80 || scrollingUp);
      previousScrollY.current = currentScrollY;
    };

    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  }, [menuOpen, toolsOpen]);

  const toggleTheme = () => {
    const nextDarkMode = !darkMode;
    document.documentElement.classList.toggle('dark', nextDarkMode);
    localStorage.setItem('theme', nextDarkMode ? 'dark' : 'light');
    setDarkMode(nextDarkMode);
  };

  const openRpcDialog = () => {
    setRpcUrl(getActiveRpcUrl());
    setRpcError('');
    setRpcDialogOpen(true);
  };

  const connectRpc = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const normalized = rpcUrl.trim().replace(/\/$/, '');

    try {
      const parsed = new URL(normalized);
      if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) {
        throw new Error('invalid RPC URL');
      }
      setActiveRpcUrl(normalized);
      setRpcDialogOpen(false);
      window.location.reload();
    } catch {
      setRpcError('Enter a valid HTTP or HTTPS RPC URL.');
    }
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
          <Link href="/account" onClick={() => setMenuOpen(false)}>Accounts</Link>
          <Link href="/nft" onClick={() => setMenuOpen(false)}>NFTs</Link>
          <div className={`site-nav__dropdown ${toolsOpen ? "site-nav__dropdown--open" : ""}`}>
            <button
              className="site-nav__dropdown-trigger"
              type="button"
              aria-expanded={toolsOpen}
              aria-haspopup="menu"
              onClick={() => setToolsOpen((open) => !open)}
            >
              Tools <span aria-hidden="true">⌄</span>
            </button>
            <div className="site-nav__dropdown-menu" role="menu">
              <Link href="/modules" role="menuitem" onClick={() => { setMenuOpen(false); setToolsOpen(false); }}>Modules</Link>
              <Link href="/checkpoint-object-graph" role="menuitem" onClick={() => { setMenuOpen(false); setToolsOpen(false); }}>Checkpoint Graph</Link>
              <Link href="/smt" role="menuitem" onClick={() => { setMenuOpen(false); setToolsOpen(false); }}>SMT Status</Link>
            </div>
          </div>
        </nav>
        <div className="site-header__actions">
          <button className="theme-toggle" type="button" onClick={toggleTheme} aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}>
            {darkMode ? (
              <svg aria-hidden="true" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
            ) : (
              <svg aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M20.5 14.4A8 8 0 0 1 9.6 3.5 8.5 8.5 0 1 0 20.5 14.4Z" /></svg>
            )}
          </button>
          <button className="rpc-selector-button" type="button" onClick={openRpcDialog} aria-haspopup="dialog">
            RPC
          </button>
          <button
            className="menu-toggle"
            type="button"
            aria-label="Toggle navigation"
            aria-expanded={menuOpen}
            onClick={() => {
              setMenuOpen((open) => !open);
              setToolsOpen(false);
            }}
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
      {rpcDialogOpen ? (
        <div className="rpc-dialog-backdrop" role="presentation" onMouseDown={() => setRpcDialogOpen(false)}>
          <section
            className="rpc-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="rpc-dialog-title"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="rpc-dialog__header">
              <p id="rpc-dialog-title">Choose RPC endpoint</p>
              <button type="button" onClick={() => setRpcDialogOpen(false)} aria-label="Close RPC selector">Close</button>
            </div>
            <form onSubmit={connectRpc}>
              <p className="rpc-dialog__copy">Choose the Kanari RPC node this explorer should read. Your selection is saved in this browser.</p>
              <div className="rpc-preset-list" role="group" aria-label="RPC endpoint presets">
                {RPC_PRESETS.map((preset) => {
                  const selected = rpcUrl.trim().replace(/\/$/, '') === preset.url;
                  return (
                    <button
                      className={`rpc-preset ${selected ? 'rpc-preset--selected' : ''}`}
                      type="button"
                      key={preset.url}
                      aria-pressed={selected}
                      onClick={() => {
                        setRpcUrl(preset.url);
                        setRpcError('');
                      }}
                    >
                      <span>{preset.name}</span>
                      <small>{preset.url}</small>
                    </button>
                  );
                })}
              </div>
              <label htmlFor="custom-rpc-url">Custom RPC URL</label>
              <input
                id="custom-rpc-url"
                type="url"
                inputMode="url"
                autoFocus
                required
                placeholder="http://127.0.0.1:6767"
                value={rpcUrl}
                onChange={(event) => setRpcUrl(event.target.value)}
              />
              {rpcError ? <p className="rpc-dialog__error" role="alert">{rpcError}</p> : null}
              <div className="rpc-dialog__actions">
                <button type="button" onClick={() => setRpcDialogOpen(false)}>Cancel</button>
                <button className="rpc-dialog__connect" type="submit">Connect</button>
              </div>
            </form>
          </section>
        </div>
      ) : null}
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
