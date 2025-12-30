'use client'

import { useState } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'

const navigation = [
  {
    title: 'Getting Started',
    items: [
      { title: 'Introduction', href: '/docs' },
      { title: 'Quick Start', href: '/docs/getting-started' },
      { title: 'Installation', href: '/docs/installation' },
    ],
  },
  {
    title: 'Core Concepts',
    items: [
      { title: 'Functions', href: '/docs/functions' },
      { title: 'Handler Parameters', href: '/docs/handler-parameters' },
      { title: 'Runtimes', href: '/docs/runtimes' },
    ],
  },
  {
    title: 'API',
    items: [
      { title: 'API Reference', href: '/docs/api-reference' },
      { title: 'Authentication', href: '/docs/authentication' },
      { title: 'Error Handling', href: '/docs/errors' },
    ],
  },
  {
    title: 'Guides',
    items: [
      { title: 'REST API', href: '/docs/guides/rest-api' },
      { title: 'Webhooks', href: '/docs/guides/webhooks' },
      { title: 'Scheduled Jobs', href: '/docs/guides/scheduled-jobs' },
    ],
  },
  {
    title: 'Languages',
    items: [
      { title: 'Python', href: '/docs/languages/python' },
      { title: 'Node.js', href: '/docs/languages/nodejs' },
      { title: 'Java', href: '/docs/languages/java' },
    ],
  },
]

export function DocsSidebar() {
  const pathname = usePathname()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  return (
    <>
      {/* Mobile menu button */}
      <button
        onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
        className="lg:hidden fixed bottom-4 right-4 z-50 bg-blue-600 text-white p-4 rounded-full shadow-lg"
      >
        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
        </svg>
      </button>

      {/* Sidebar */}
      <aside
        className={`${
          mobileMenuOpen ? 'translate-x-0' : '-translate-x-full'
        } lg:translate-x-0 fixed lg:sticky top-0 left-0 z-40 w-64 h-screen lg:h-[calc(100vh-4rem)] overflow-y-auto bg-gray-900 border-r border-gray-800 transition-transform duration-300`}
      >
        <div className="p-6">
          <div className="flex items-center justify-between mb-6 lg:hidden">
            <span className="text-white font-semibold">Documentation</span>
            <button onClick={() => setMobileMenuOpen(false)} className="text-gray-400">
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <nav className="space-y-8">
            {navigation.map((section) => (
              <div key={section.title}>
                <h3 className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-3">
                  {section.title}
                </h3>
                <ul className="space-y-2">
                  {section.items.map((item) => {
                    const isActive = pathname === item.href
                    return (
                      <li key={item.href}>
                        <Link
                          href={item.href}
                          onClick={() => setMobileMenuOpen(false)}
                          className={`block px-3 py-2 rounded-lg text-sm transition ${
                            isActive
                              ? 'bg-blue-600 text-white font-medium'
                              : 'text-gray-300 hover:bg-gray-800 hover:text-white'
                          }`}
                        >
                          {item.title}
                        </Link>
                      </li>
                    )
                  })}
                </ul>
              </div>
            ))}
          </nav>

          <div className="mt-8 pt-8 border-t border-gray-800">
            <a
              href="https://github.com/ip888/nanolambda"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center text-sm text-gray-400 hover:text-white transition"
            >
              <svg className="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
              </svg>
              GitHub
            </a>
          </div>
        </div>
      </aside>

      {/* Overlay for mobile */}
      {mobileMenuOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-30 lg:hidden"
          onClick={() => setMobileMenuOpen(false)}
        />
      )}
    </>
  )
}
