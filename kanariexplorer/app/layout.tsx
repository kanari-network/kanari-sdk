import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";
import Link from "next/link";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" });
const mono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono" });

export const metadata: Metadata = {
  title: "KanariScan",
  description: "Kanari Network Blockchain Explorer",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.variable} ${mono.variable} font-sans bg-black text-zinc-100 min-h-screen flex flex-col selection:bg-zinc-800`}>
        {/* Navbar */}
        <header className="sticky top-0 z-50 bg-black/90 backdrop-blur-md border-b border-zinc-800">
          <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
            <Link href="/" className="flex items-center gap-3 group">
              <div className="w-8 h-8 bg-zinc-100 rounded flex items-center justify-center text-black font-black text-xl">
                K
              </div>
              <span className="text-xl font-bold tracking-wide text-zinc-100 group-hover:text-white transition-colors">
                KanariScan
              </span>
            </Link>
            <nav className="hidden md:flex space-x-1">
              <Link href="/" className="px-3 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900 rounded-md transition-all">Home</Link>
              <Link href="/tx" className="px-3 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900 rounded-md transition-all">Transactions</Link>
              <Link href="/coins" className="px-3 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900 rounded-md transition-all">Tokens</Link>
              <Link href="/account" className="px-3 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900 rounded-md transition-all">Accounts</Link>
              <Link href="/nft" className="px-3 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900 rounded-md transition-all">NFTs</Link>
            </nav>
          </div>
        </header>

        {/* Main Content */}
        <main className="flex-1 flex flex-col">
          {children}
        </main>

        {/* Footer */}
        <footer className="bg-black py-8 border-t border-zinc-900 mt-auto">
          <div className="max-w-7xl mx-auto px-6 flex flex-col md:flex-row justify-between items-center gap-4 text-xs font-medium text-zinc-600">
            <div>© {new Date().getFullYear()} Kanari Network. All rights reserved.</div>
            <div className="flex gap-4">
              <a href="#" className="hover:text-zinc-300 transition-colors">Twitter</a>
              <a href="#" className="hover:text-zinc-300 transition-colors">Discord</a>
              <a href="#" className="hover:text-zinc-300 transition-colors">GitHub</a>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}