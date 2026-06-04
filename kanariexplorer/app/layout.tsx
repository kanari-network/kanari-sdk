import type { Metadata } from "next";
import { PageShell } from "./components/SiteChrome";
import "./globals.css";

export const metadata: Metadata = {
  title: "Kanari Explorer",
  description: "Network explorer for Kanari blocks, accounts, tokens, transactions, and NFTs.",
  icons: {
    icon: ['/icons/favicon.ico?v=4'],
    apple: ['/icons/apple-touch-icon.png?v=4'],
    shortcut: ['/icons/apple-touch-icon.png'],
  }
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: `try{var t=localStorage.getItem('theme');var d=t?t==='dark':window.matchMedia('(prefers-color-scheme: dark)').matches;document.documentElement.classList.toggle('dark',d)}catch(e){}` }} />
      </head>
      <body>
        <div className="site-shell">
          <PageShell>
            <body>{children}</body>
          </PageShell>
        </div>
      </body>
    </html>
  );
}
