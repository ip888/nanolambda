'use client'

import Link from 'next/link'
import { useState } from 'react'

export function Header() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  return (
    <header className="fixed top-0 w-full bg-black/80 backdrop-blur-md border-b border-gray-800 z-50">
      <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          {/* Logo */}
          <Link href="/" className="flex items-center space-x-2">
            <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center">
              <span className="text-white font-bold text-xl">λ</span>
            </div>
            <span className="text-xl font-bold text-white">NanoLambda</span>
          </Link>

          {/* Desktop Navigation */}
          <div className="hidden md:flex items-center space-x-8">
            <Link href="/docs" className="text-gray-300 hover:text-white transition">
              Docs
            </Link>
            <Link href="/play" className="text-gray-300 hover:text-white transition">
              Playground
            </Link>
            <Link href="/pricing" className="text-gray-300 hover:text-white transition">
              Pricing
            </Link>
            <Link
              href="/dashboard"
              className="text-gray-300 hover:text-white transition"
            >
              Sign In
            </Link>
            <Link
              href="/dashboard"
              className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg transition"
            >
              Start Free
            </Link>
          </div>

          {/* Mobile menu button */}
          <button
            className="md:hidden text-gray-300"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
        </div>

        {/* Mobile menu */}
        {mobileMenuOpen && (
          <div className="md:hidden py-4 space-y-3">
            <Link href="/docs" className="block text-gray-300 hover:text-white transition">
              Docs
            </Link>
            <Link href="/play" className="block text-gray-300 hover:text-white transition">
              Playground
            </Link>
            <Link href="/pricing" className="block text-gray-300 hover:text-white transition">
              Pricing
            </Link>
            <Link href="/dashboard" className="block text-gray-300 hover:text-white transition">
              Sign In
            </Link>
            <Link
              href="/dashboard"
              className="block bg-blue-600 text-white px-4 py-2 rounded-lg text-center"
            >
              Start Free
            </Link>
          </div>
        )}
      </nav>
    </header>
  )
}
