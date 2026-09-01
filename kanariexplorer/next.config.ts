import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Vercel injects Turbopack via modifiedConfig – standalone + Turbopack does not create .nft.json file that Vercel searches for.
  ...(process.env.VERCEL ? {} : { output: 'standalone' as const }),
};

export default nextConfig;
