import type { Metadata } from "next";
import Link from "next/link";
import MobileNav from "./components/MobileNav";
import "./globals.css";

export const metadata: Metadata = {
  title: "KanariScan",
  description: "Kanari Network Blockchain Explorer",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="relative flex min-h-screen flex-col overflow-x-hidden bg-[#09090b] font-sans text-zinc-300 selection:bg-emerald-500/30 selection:text-emerald-200">
        <div className="pointer-events-none absolute inset-x-0 top-0 z-0 h-96 bg-linear-to-b from-emerald-500/5 via-cyan-500/5 to-transparent" />

        <header className="sticky top-0 z-50 border-b border-white/5 bg-[#09090b]/70 shadow-sm backdrop-blur-xl">
          <div className="mx-auto flex max-w-7xl items-center justify-between px-6 py-4">
            <Link href="/" className="group relative z-10 flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-md bg-linear-to-br from-emerald-400 to-cyan-500 text-xl font-black text-white shadow-lg shadow-emerald-500/20 transition-all group-hover:shadow-emerald-500/40">
                K
              </div>
              <span className="text-xl font-bold tracking-normal text-white">
                Kanari<span className="font-normal text-zinc-500">Scan</span>
              </span>
            </Link>
            <nav className="relative z-10 hidden items-center space-x-2 md:flex">
              {[
                { name: "Home", path: "/" },
                { name: "Transactions", path: "/tx" },
                { name: "Tokens", path: "/coins" },
                { name: "Accounts", path: "/account" },
                { name: "NFTs", path: "/nft" },
              ].map((item) => (
                <Link
                  key={item.name}
                  href={item.path}
                  className="rounded-md px-4 py-2 text-sm font-medium text-zinc-400 transition-all duration-200 hover:bg-white/5 hover:text-white"
                >
                  {item.name}
                </Link>
              ))}
            </nav>
            <MobileNav />
          </div>
        </header>

        <main className="relative z-10 flex flex-1 flex-col">{children}</main>

        <footer className="relative z-10 mt-auto border-t border-white/5 bg-[#09090b] py-8">
          <div className="mx-auto flex max-w-7xl flex-col items-center justify-between gap-4 px-6 text-sm font-medium text-zinc-500 md:flex-row">
            <div>&copy; {new Date().getFullYear()} Kanari Network. All rights reserved.</div>
            <div className="flex gap-6">
              <a href="#" className="transition-colors hover:text-emerald-400">
                Twitter
              </a>
              <a href="#" className="transition-colors hover:text-emerald-400">
                Discord
              </a>
              <a href="#" className="transition-colors hover:text-emerald-400">
                GitHub
              </a>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}
