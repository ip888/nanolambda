import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "NanoLambda - Zero-Latency Serverless Platform",
  description: "Deploy functions that respond in 0ms. No cold starts. No complexity. Just speed.",
  keywords: ["serverless", "lambda", "cloud functions", "zero latency", "fast cold starts"],
  authors: [{ name: "NanoLambda" }],
  openGraph: {
    title: "NanoLambda - Zero-Latency Serverless",
    description: "Deploy functions that respond in 0ms. No cold starts.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} antialiased bg-black text-white`}>
        {children}
      </body>
    </html>
  );
}
