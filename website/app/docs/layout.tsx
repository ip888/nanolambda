import { DocsSidebar } from '@/components/docs/DocsSidebar'
import Link from 'next/link'

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <div className="min-h-screen bg-black">
      {/* Header */}
      <header className="sticky top-0 z-50 bg-black/80 backdrop-blur-md border-b border-gray-800">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <Link href="/" className="flex items-center space-x-2">
              <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center">
                <span className="text-white font-bold text-xl">λ</span>
              </div>
              <span className="text-xl font-bold text-white">NanoLambda</span>
            </Link>

            <nav className="hidden md:flex items-center space-x-6">
              <Link href="/docs" className="text-blue-400 font-medium">
                Docs
              </Link>
              <Link href="/play" className="text-gray-300 hover:text-white transition">
                Playground
              </Link>
              <Link
                href="/dashboard"
                className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg transition"
              >
                Dashboard
              </Link>
            </nav>
          </div>
        </div>
      </header>

      {/* Main content */}
      <div className="flex max-w-7xl mx-auto">
        <DocsSidebar />
        <main className="flex-1 lg:pl-8">
          {children}
        </main>
      </div>
    </div>
  )
}
