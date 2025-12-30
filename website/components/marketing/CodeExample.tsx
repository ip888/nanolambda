'use client'

import { useState } from 'react'

const examples = [
  {
    language: 'Python',
    code: `def handler(event, context):
    name = event.get('name', 'World')
    return {
        'message': f'Hello, {name}!',
        'latency': '0ms'
    }`,
    payload: { name: 'Alice' },
    response: {
      message: 'Hello, Alice!',
      latency: '0ms',
      cold_start: false,
      execution_time_ms: 0,
    },
  },
  {
    language: 'Node.js',
    code: `exports.handler = async (event) => {
    const name = event.name || 'World';
    return {
        message: \`Hello, \${name}!\`,
        latency: '0ms'
    };
};`,
    payload: { name: 'Bob' },
    response: {
      message: 'Hello, Bob!',
      latency: '0ms',
      cold_start: false,
      execution_time_ms: 0,
    },
  },
]

export function CodeExample() {
  const [selectedLang, setSelectedLang] = useState(0)
  const [isRunning, setIsRunning] = useState(false)
  const [showOutput, setShowOutput] = useState(false)

  const example = examples[selectedLang]

  const handleRun = () => {
    setIsRunning(true)
    setShowOutput(false)
    
    // Simulate instant execution
    setTimeout(() => {
      setIsRunning(false)
      setShowOutput(true)
    }, 100)
  }

  return (
    <section className="py-20 px-4 sm:px-6 lg:px-8 bg-black">
      <div className="max-w-7xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-4xl md:text-5xl font-bold text-white mb-4">
            Deploy in Seconds
          </h2>
          <p className="text-xl text-gray-400">
            Write code, deploy instantly, get 0ms responses
          </p>
        </div>

        <div className="grid md:grid-cols-2 gap-8">
          {/* Code Editor */}
          <div className="bg-gray-900 rounded-2xl overflow-hidden border border-gray-800">
            <div className="flex items-center justify-between bg-gray-800 px-6 py-3 border-b border-gray-700">
              <div className="flex space-x-2">
                {examples.map((ex, i) => (
                  <button
                    key={ex.language}
                    onClick={() => {
                      setSelectedLang(i)
                      setShowOutput(false)
                    }}
                    className={`px-4 py-2 rounded-lg text-sm font-medium transition ${
                      selectedLang === i
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                    }`}
                  >
                    {ex.language}
                  </button>
                ))}
              </div>
              <button
                onClick={handleRun}
                disabled={isRunning}
                className="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-lg text-sm font-medium transition flex items-center space-x-2"
              >
                {isRunning ? (
                  <>
                    <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                    </svg>
                    <span>Running...</span>
                  </>
                ) : (
                  <>
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <span>Run</span>
                  </>
                )}
              </button>
            </div>

            <div className="p-6">
              <pre className="text-gray-300 font-mono text-sm leading-relaxed">
                <code>{example.code}</code>
              </pre>
            </div>

            <div className="border-t border-gray-800 px-6 py-4">
              <div className="text-gray-400 text-xs font-mono mb-2">Input payload:</div>
              <pre className="text-blue-400 font-mono text-sm">
                {JSON.stringify(example.payload, null, 2)}
              </pre>
            </div>
          </div>

          {/* Output Panel */}
          <div className="bg-gray-900 rounded-2xl overflow-hidden border border-gray-800">
            <div className="bg-gray-800 px-6 py-3 border-b border-gray-700">
              <h3 className="text-white font-semibold">Response</h3>
            </div>

            {showOutput ? (
              <div className="p-6 space-y-4">
                <div className="flex items-center space-x-3 text-green-400">
                  <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                  </svg>
                  <span className="font-semibold">Success</span>
                </div>

                <div>
                  <div className="text-gray-400 text-xs font-mono mb-2">Output:</div>
                  <pre className="bg-gray-800 p-4 rounded-lg text-gray-300 font-mono text-sm overflow-x-auto">
                    {JSON.stringify(example.response, null, 2)}
                  </pre>
                </div>

                <div className="grid grid-cols-2 gap-4 pt-4 border-t border-gray-800">
                  <div>
                    <div className="text-gray-400 text-xs mb-1">Execution Time</div>
                    <div className="text-2xl font-bold text-green-400">0ms</div>
                  </div>
                  <div>
                    <div className="text-gray-400 text-xs mb-1">Cold Start</div>
                    <div className="text-2xl font-bold text-green-400">No</div>
                  </div>
                </div>

                <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-lg">
                  <p className="text-green-400 text-sm flex items-center">
                    <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M13 10V3L4 14h7v7l9-11h-7z" clipRule="evenodd" />
                    </svg>
                    Instant response with 0ms latency
                  </p>
                </div>
              </div>
            ) : (
              <div className="p-6 flex items-center justify-center h-96">
                <div className="text-center text-gray-500">
                  <svg className="w-16 h-16 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <p>Click "Run" to see the output</p>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="mt-12 text-center">
          <a
            href="/play"
            className="inline-flex items-center text-blue-400 hover:text-blue-300 font-semibold transition"
          >
            Try more examples in the playground
            <svg className="w-5 h-5 ml-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 8l4 4m0 0l-4 4m4-4H3" />
            </svg>
          </a>
        </div>
      </div>
    </section>
  )
}
