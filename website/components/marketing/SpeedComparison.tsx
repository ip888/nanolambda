'use client'

export function SpeedComparison() {
  const platforms = [
    { name: 'NanoLambda', warmStart: 0, color: 'bg-green-500', highlight: true },
    { name: 'AWS Lambda', warmStart: 75, color: 'bg-orange-500' },
    { name: 'Google Cloud', warmStart: 60, color: 'bg-blue-500' },
    { name: 'Azure Functions', warmStart: 90, color: 'bg-cyan-500' },
    { name: 'Cloudflare', warmStart: 20, color: 'bg-yellow-500' },
  ]

  const maxValue = Math.max(...platforms.map(p => p.warmStart))

  return (
    <section className="py-20 px-4 sm:px-6 lg:px-8 bg-gray-950">
      <div className="max-w-7xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-4xl md:text-5xl font-bold text-white mb-4">
            Why Choose NanoLambda?
          </h2>
          <p className="text-xl text-gray-400">
            Consistently faster warm starts than any competitor
          </p>
        </div>

        <div className="bg-gray-900 rounded-2xl p-8 md:p-12 border border-gray-800">
          <h3 className="text-2xl font-bold text-white mb-8">
            Warm Start Latency Comparison
          </h3>

          <div className="space-y-6">
            {platforms.map((platform) => (
              <div key={platform.name}>
                <div className="flex items-center justify-between mb-2">
                  <span className={`font-semibold ${platform.highlight ? 'text-green-400' : 'text-gray-300'}`}>
                    {platform.name}
                  </span>
                  <span className={`font-bold ${platform.highlight ? 'text-green-400' : 'text-gray-400'}`}>
                    {platform.warmStart}ms
                  </span>
                </div>
                <div className="relative h-8 bg-gray-800 rounded-lg overflow-hidden">
                  <div
                    className={`h-full ${platform.color} transition-all duration-1000 ease-out flex items-center justify-end pr-4`}
                    style={{ width: platform.warmStart === 0 ? '5%' : `${(platform.warmStart / maxValue) * 100}%` }}
                  >
                    {platform.highlight && (
                      <span className="text-white text-sm font-bold">⚡ Instant</span>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>

          <div className="mt-8 p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
            <p className="text-gray-300 text-sm">
              <span className="text-blue-400 font-semibold">Benchmark methodology:</span> Average warm start latency across 1,000 requests. 
              Measured with simple "Hello World" function. Data collected December 2025.
            </p>
          </div>
        </div>

        {/* Key Benefits */}
        <div className="grid md:grid-cols-3 gap-8 mt-16">
          <div className="text-center">
            <div className="w-16 h-16 bg-green-500/10 rounded-2xl flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
            <h3 className="text-xl font-bold text-white mb-2">Instant Response</h3>
            <p className="text-gray-400">
              0ms warm starts mean your APIs respond instantly, every time.
            </p>
          </div>

          <div className="text-center">
            <div className="w-16 h-16 bg-blue-500/10 rounded-2xl flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <h3 className="text-xl font-bold text-white mb-2">Lower Costs</h3>
            <p className="text-gray-400">
              Faster execution means less compute time and lower bills.
            </p>
          </div>

          <div className="text-center">
            <div className="w-16 h-16 bg-purple-500/10 rounded-2xl flex items-center justify-center mx-auto mb-4">
              <svg className="w-8 h-8 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 10h4.764a2 2 0 011.789 2.894l-3.5 7A2 2 0 0115.263 21h-4.017c-.163 0-.326-.02-.485-.06L7 20m7-10V5a2 2 0 00-2-2h-.095c-.5 0-.905.405-.905.905 0 .714-.211 1.412-.608 2.006L7 11v9m7-10h-2M7 20H5a2 2 0 01-2-2v-6a2 2 0 012-2h2.5" />
              </svg>
            </div>
            <h3 className="text-xl font-bold text-white mb-2">Better UX</h3>
            <p className="text-gray-400">
              Users get immediate feedback. No loading spinners.
            </p>
          </div>
        </div>
      </div>
    </section>
  )
}
