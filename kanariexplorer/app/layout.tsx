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
      <body className={`${inter.variable} ${mono.variable} font-sans bg-[#09090b] text-zinc-300 min-h-screen flex flex-col selection:bg-emerald-500/30 selection:text-emerald-200 relative`}>
        {/* Background Glow Effect */}
        <div className="absolute top-0 inset-x-0 h-125 bg-linear-to-b from-emerald-500/5 via-cyan-500/5 to-transparent pointer-events-none z-0"></div>

        {/* Navbar */}
        <header className="sticky top-0 z-50 bg-[#09090b]/70 backdrop-blur-xl border-b border-white/5 shadow-sm">
          <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
            <Link href="/" className="flex items-center gap-3 group relative z-10">
              <div className="w-9 h-9 bg-linear-to-br from-emerald-400 to-cyan-500 rounded-xl flex items-center justify-center text-white font-black text-xl shadow-lg shadow-emerald-500/20 group-hover:shadow-emerald-500/40 transition-all">
                K
              </div>
              <span className="text-xl font-bold tracking-tight text-white">
                Kanari<span className="text-zinc-500 font-normal">Scan</span>
              </span>
            </Link>
            <nav className="hidden md:flex items-center space-x-2 relative z-10">
              {[
                { name: 'Home', path: '/' },
                { name: 'Transactions', path: '/tx' },
                { name: 'Tokens', path: '/coins' },
                { name: 'Accounts', path: '/account' },
                { name: 'NFTs', path: '/nft' }
              ].map((item) => (
                <Link key={item.name} href={item.path} className="px-4 py-2 text-sm font-medium text-zinc-400 hover:text-white hover:bg-white/5 rounded-lg transition-all duration-200">
                  {item.name}
                </Link>
              ))}
            </nav>
          </div>
        </header>

        {/* Main Content */}
        <main className="flex-1 flex flex-col relative z-10">
          {children}
        </main>

        {/* Footer */}
        <footer className="bg-[#09090b] py-8 border-t border-white/5 mt-auto relative z-10">
          <div className="max-w-7xl mx-auto px-6 flex flex-col md:flex-row justify-between items-center gap-4 text-sm font-medium text-zinc-500">
            <div>© {new Date().getFullYear()} Kanari Network. All rights reserved.</div>
            <div className="flex gap-6">
              <a href="#" className="hover:text-emerald-400 transition-colors">Twitter</a>
              <a href="#" className="hover:text-emerald-400 transition-colors">Discord</a>
              <a href="#" className="hover:text-emerald-400 transition-colors">GitHub</a>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}