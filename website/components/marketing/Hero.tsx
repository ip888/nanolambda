import Link from 'next/link'

export function Hero() {
  return (
    <section className="relative pt-32 pb-20 px-4 sm:px-6 lg:px-8 overflow-hidden">
      {/* Gradient background */}
      <div className="absolute inset-0 bg-gradient-to-br from-blue-900/20 via-purple-900/20 to-black" />
      
      {/* Grid pattern */}
      <div className="absolute inset-0 bg-[url('/grid.svg')] opacity-10" />
      
      <div className="relative max-w-7xl mx-auto">
        <div className="text-center">
          {/* Badge */}
          <div className="inline-flex items-center px-4 py-2 bg-blue-500/10 border border-blue-500/20 rounded-full mb-8">
            <span className="text-blue-400 text-sm font-medium">
              ⚡ 0ms Cold Starts · Open Source
            </span>
          </div>

          {/* Main heading */}
          <h1 className="text-5xl md:text-7xl font-bold text-white mb-6 leading-tight">
            Zero-Latency
            <br />
            <span className="bg-gradient-to-r from-blue-400 to-purple-500 text-transparent bg-clip-text">
              Serverless
            </span>
          </h1>

          {/* Subheading */}
          <p className="text-xl md:text-2xl text-gray-300 mb-12 max-w-3xl mx-auto">
            Deploy functions that respond in <span className="text-green-400 font-bold">0ms</span>.
            <br />
            No cold starts. No complexity. Just speed.
          </p>

          {/* CTA Buttons */}
          <div className="flex flex-col sm:flex-row gap-4 justify-center mb-12">
            <Link
              href="/dashboard"
              className="bg-blue-600 hover:bg-blue-700 text-white px-8 py-4 rounded-lg text-lg font-semibold transition shadow-lg shadow-blue-500/50"
            >
              Start Free →
            </Link>
            <Link
              href="/play"
              className="bg-gray-800 hover:bg-gray-700 text-white px-8 py-4 rounded-lg text-lg font-semibold transition border border-gray-700"
            >
              Try Playground
            </Link>
            <Link
              href="/docs"
              className="bg-transparent hover:bg-gray-800/50 text-white px-8 py-4 rounded-lg text-lg font-semibold transition border border-gray-700"
            >
              View Docs
            </Link>
          </div>

          {/* Stats */}
          <div className="grid grid-cols-3 gap-8 max-w-2xl mx-auto pt-8 border-t border-gray-800">
            <div>
              <div className="text-3xl font-bold text-green-400">0ms</div>
              <div className="text-sm text-gray-400 mt-1">Warm Start</div>
            </div>
            <div>
              <div className="text-3xl font-bold text-blue-400">3</div>
              <div className="text-sm text-gray-400 mt-1">Languages</div>
            </div>
            <div>
              <div className="text-3xl font-bold text-purple-400">100%</div>
              <div className="text-sm text-gray-400 mt-1">Open Source</div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
