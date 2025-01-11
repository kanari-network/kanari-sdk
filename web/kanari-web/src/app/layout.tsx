import type { Metadata } from "next";
import { SpeedInsights } from "@vercel/speed-insights/next"
import { Analytics } from "@vercel/analytics/react"
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Kanari Network",
  description: "Kanari Network is a modern bank card for a modern world",
  icons: {
    icon: ['/icons/favicon.ico?v=4'],
    apple:['/icons/apple-touch-icon.png?v=4'],
    shortcut:['/icons/apple-touch-icon.png'],
  }
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={inter.className}>{children}</body>
      <SpeedInsights />
      <Analytics />
    </html>
  );
}
