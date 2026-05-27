"use client";

import Link from "next/link";
import { useState } from "react";

const navItems = [
  { name: "Home", path: "/" },
  { name: "Transactions", path: "/tx" },
  { name: "Tokens", path: "/coins" },
  { name: "Accounts", path: "/account" },
  { name: "NFTs", path: "/nft" },
];

export default function MobileNav() {
  const [open, setOpen] = useState(false);

  return (
    <div className="relative z-20 md:hidden">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-label="Open navigation menu"
        className="inline-flex h-10 items-center gap-2 rounded-xl border border-white/10 bg-white/[0.04] px-3 text-xs font-black uppercase tracking-widest text-zinc-200 shadow-lg shadow-black/20 transition hover:border-emerald-400/30 hover:bg-emerald-400/10"
      >
        <span className="flex h-4 w-4 flex-col justify-center gap-1">
          <span className={`h-0.5 rounded-full bg-current transition ${open ? "translate-y-1.5 rotate-45" : ""}`} />
          <span className={`h-0.5 rounded-full bg-current transition ${open ? "opacity-0" : ""}`} />
          <span className={`h-0.5 rounded-full bg-current transition ${open ? "-translate-y-1.5 -rotate-45" : ""}`} />
        </span>
        Menu
      </button>

      {open && (
        <>
          <button
            type="button"
            aria-label="Close navigation menu"
            className="fixed inset-0 z-10 cursor-default bg-black/20"
            onClick={() => setOpen(false)}
          />
          <div className="absolute right-0 top-12 z-20 w-56 overflow-hidden rounded-2xl border border-white/10 bg-[#111113]/95 p-2 shadow-2xl shadow-black/40 backdrop-blur-xl">
            {navItems.map((item) => (
              <Link
                key={item.path}
                href={item.path}
                onClick={() => setOpen(false)}
                className="flex items-center justify-between rounded-xl px-4 py-3 text-sm font-bold text-zinc-300 transition hover:bg-emerald-400/10 hover:text-emerald-300"
              >
                {item.name}
                <span className="text-zinc-700">&gt;</span>
              </Link>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
