import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "maenifold",
  description: "Context engineering infrastructure for AI agents.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
