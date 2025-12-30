'use client'

import { useState } from 'react'
import Editor from '@monaco-editor/react'
import Link from 'next/link'

const examples = {
  'hello-python': {
    name: 'Hello World (Python)',
    language: 'python',
    runtime: 'python3.12',
    code: `def handler(event, context):
    name = event.get('name', 'World')
    return {
        'message': f'Hello, {name}!',
        'request_id': context.request_id
    }`,
    payload: { name: 'Alice' },
  },
  'hello-node': {
    name: 'Hello World (Node.js)',
    language: 'javascript',
    runtime: 'nodejs22',
    code: `exports.handler = async (event, context) => {
    const name = event.name || 'World';
    return {
        message: \`Hello, \${name}!\`,
        request_id: context.request_id
    };
};`,
    payload: { name: 'Bob' },
  },
  'calculator': {
    name: 'Calculator (Python)',
    language: 'python',
    runtime: 'python3.12',
    code: `def handler(event, context):
    a = event.get('a', 0)
    b = event.get('b', 0)
    operation = event.get('operation', 'add')
    
    operations = {
        'add': a + b,
        'subtract': a - b,
        'multiply': a * b,
        'divide': a / b if b != 0 else 'Error: Division by zero'
    }
    
    result = operations.get(operation, 'Unknown operation')
    
    return {
        'a': a,
        'b': b,
        'operation': operation,
        'result': result
    }`,
    payload: { a: 10, b: 5, operation: 'multiply' },
  },
  'data-processing': {
    name: 'Data Processing (Node.js)',
    language: 'javascript',
    runtime: 'nodejs22',
    code: `exports.handler = async (event, context) => {
    const data = event.data || [];
    
    const stats = {
        count: data.length,
        sum: data.reduce((a, b) => a + b, 0),
        avg: data.length > 0 ? data.reduce((a, b) => a + b, 0) / data.length : 0,
        min: data.length > 0 ? Math.min(...data) : null,
        max: data.length > 0 ? Math.max(...data) : null
    };
    
    return {
        processed_at: new Date().toISOString(),
        stats: stats,
        processed_by: context.function_name
    };
};`,
    payload: { data: [1, 2, 3, 4, 5, 10, 20, 15] },
  },
}

export default function PlaygroundPage() {
  const [selectedExample, setSelectedExample] = useState<keyof typeof examples>('hello-python')
  const [code, setCode] = useState(examples[selectedExample].code)
  const [payload, setPayload] = useState(JSON.stringify(examples[selectedExample].payload, null, 2))
  const [output, setOutput] = useState('')
  const [isRunning, setIsRunning] = useState(false)
  const [error, setError] = useState('')

  const example = examples[selectedExample]

  const handleExampleChange = (key: keyof typeof examples) => {
    setSelectedExample(key)
    setCode(examples[key].code)
    setPayload(JSON.stringify(examples[key].payload, null, 2))
    setOutput('')
    setError('')
  }

  const runCode = async () => {
    setIsRunning(true)
    setError('')
    setOutput('')

    const functionName = `playground-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
    const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

    try {
      // Parse the payload
      let payloadData
      try {
        payloadData = JSON.parse(payload)
      } catch (e) {
        throw new Error('Invalid JSON payload')
      }

      // Get the current example to determine runtime
      const currentExample = examples[selectedExample as keyof typeof examples]
      const runtime = currentExample.runtime

      // Demo API key for playground (public demo key)
      const DEMO_API_KEY = 'nl_683368351d5d230d1a4853395c64daa9663f9f374015e29024f98561ef618a7d'

      // Step 1: Create the function
      const createResponse = await fetch(`${API_BASE}/functions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${DEMO_API_KEY}`,
        },
        body: JSON.stringify({
          name: functionName,
          runtime: runtime,
          handler: 'handler', // Function entry point
          code: code,
          memory_mb: 128,
          timeout_ms: 30000, // 30 seconds in milliseconds
        }),
      })

      if (!createResponse.ok) {
        const errorData = await createResponse.text()
        throw new Error(`Failed to create function: ${errorData}`)
      }

      // Step 2: Invoke the function
      const invokeResponse = await fetch(`${API_BASE}/functions/${functionName}/invoke`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${DEMO_API_KEY}`,
        },
        body: JSON.stringify({
          payload: payloadData, // Wrap payload in object
        }),
      })

      if (!invokeResponse.ok) {
        const errorData = await invokeResponse.text()
        throw new Error(`Failed to invoke function: ${errorData}`)
      }

      const result = await invokeResponse.json()
      setOutput(JSON.stringify(result, null, 2))

      // Step 3: Clean up - delete the temporary function
      fetch(`${API_BASE}/functions/${functionName}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${DEMO_API_KEY}`,
        },
      }).catch((e) => {
        console.warn('Failed to clean up temporary function:', e)
      })

      setIsRunning(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred')
      setIsRunning(false)

      // Try to clean up on error
      fetch(`${API_BASE}/functions/${functionName}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${DEMO_API_KEY}`,
        },
      }).catch(() => {
        // Ignore cleanup errors
      })
    }
  }

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
              <span className="text-gray-400">Playground</span>
            </Link>

            <nav className="hidden md:flex items-center space-x-6">
              <Link href="/docs" className="text-gray-300 hover:text-white transition">
                Docs
              </Link>
              <Link
                href="/dashboard"
                className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg transition"
              >
                Sign Up to Deploy
              </Link>
            </nav>
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto p-4 sm:p-6 lg:p-8">
        {/* Examples selector */}
        <div className="mb-6">
          <h2 className="text-2xl font-bold text-white mb-4">Interactive Playground</h2>
          <p className="text-gray-400 mb-3">
            Try NanoLambda functions directly in your browser. Write your own code or select an example below.
          </p>
          <div className="bg-green-900/20 border border-green-500/30 rounded-lg px-4 py-2 mb-4">
            <p className="text-green-300 text-sm">
              ⚡ <strong>Live Execution:</strong> This playground runs your code on a real NanoLambda instance. 
              Experience actual sub-millisecond cold starts and see real performance metrics.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">{(Object.keys(examples) as Array<keyof typeof examples>).map((key) => (
              <button
                key={key}
                onClick={() => handleExampleChange(key)}
                className={`px-4 py-2 rounded-lg text-sm font-medium transition ${
                  selectedExample === key
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
                }`}
              >
                {examples[key].name}
              </button>
            ))}
          </div>
        </div>

        {/* Code and output panels */}
        <div className="grid lg:grid-cols-2 gap-6">
          {/* Left: Code editor */}
          <div className="space-y-6">
            {/* Code editor */}
            <div className="bg-gray-900 rounded-2xl overflow-hidden border border-gray-800">
              <div className="bg-gray-800 px-6 py-3 border-b border-gray-700">
                <div className="flex items-center justify-between">
                  <span className="text-white font-semibold">
                    Function Code ({example.runtime})
                  </span>
                  <span className="text-xs text-gray-400">
                    {example.language}
                  </span>
                </div>
              </div>
              <Editor
                height="400px"
                language={example.language}
                value={code}
                onChange={(value) => setCode(value || '')}
                theme="vs-dark"
                options={{
                  minimap: { enabled: false },
                  fontSize: 14,
                  lineNumbers: 'on',
                  scrollBeyondLastLine: false,
                  automaticLayout: true,
                }}
              />
            </div>

            {/* Payload input */}
            <div className="bg-gray-900 rounded-2xl overflow-hidden border border-gray-800">
              <div className="bg-gray-800 px-6 py-3 border-b border-gray-700">
                <span className="text-white font-semibold">Input Payload (JSON)</span>
              </div>
              <Editor
                height="200px"
                language="json"
                value={payload}
                onChange={(value) => setPayload(value || '{}')}
                theme="vs-dark"
                options={{
                  minimap: { enabled: false },
                  fontSize: 14,
                  lineNumbers: 'on',
                  scrollBeyondLastLine: false,
                  automaticLayout: true,
                }}
              />
            </div>

            {/* Run button */}
            <button
              onClick={runCode}
              disabled={isRunning}
              className="w-full bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white px-6 py-4 rounded-lg font-semibold transition flex items-center justify-center space-x-2"
            >
              {isRunning ? (
                <>
                  <svg className="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                  </svg>
                  <span>Running...</span>
                </>
              ) : (
                <>
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span>Run Function</span>
                </>
              )}
            </button>
          </div>

          {/* Right: Output panel */}
          <div className="space-y-6">
            {/* Output */}
            <div className="bg-gray-900 rounded-2xl overflow-hidden border border-gray-800 h-[650px] flex flex-col">
              <div className="bg-gray-800 px-6 py-3 border-b border-gray-700">
                <span className="text-white font-semibold">Output</span>
              </div>
              <div className="flex-1 p-6 overflow-auto">
                {output ? (
                  <div className="space-y-4">
                    <div className="flex items-center space-x-3 text-green-400">
                      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                      </svg>
                      <span className="font-semibold">Success</span>
                    </div>
                    <pre className="bg-gray-800 p-4 rounded-lg text-gray-300 font-mono text-sm overflow-x-auto">
                      {output}
                    </pre>
                  </div>
                ) : error ? (
                  <div className="space-y-4">
                    <div className="flex items-center space-x-3 text-red-400">
                      <svg className="w-5 h-5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
                      </svg>
                      <span className="font-semibold">Error</span>
                    </div>
                    <div className="bg-red-900/20 border border-red-900 p-4 rounded-lg text-red-400 font-mono text-sm overflow-x-auto break-words whitespace-pre-wrap">
                      {error}
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center justify-center h-full text-center text-gray-500">
                    <div>
                      <svg className="w-16 h-16 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                      <p className="text-lg mb-2">Ready to run</p>
                      <p className="text-sm">Click "Run Function" to see the output</p>
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Info card */}
            <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4">
              <h3 className="text-blue-400 font-semibold mb-2">💡 Playground Tips</h3>
              <ul className="text-gray-300 text-sm space-y-1">
                <li>• Functions run on a real NanoLambda instance</li>
                <li>• Temporary functions are auto-deleted after execution</li>
                <li>• Sign up to deploy persistent functions with monitoring</li>
                <li>• Check out the <Link href="/docs" className="text-blue-400 hover:underline">docs</Link> for more examples</li>
              </ul>
            </div>
          </div>
        </div>

        {/* CTA */}
        <div className="mt-12 text-center p-8 bg-gray-900 rounded-2xl border border-gray-800">
          <h3 className="text-2xl font-bold text-white mb-4">
            Ready to Deploy Real Functions?
          </h3>
          <p className="text-gray-400 mb-6">
            Sign up for free and deploy production-ready functions with 0ms warm starts
          </p>
          <Link
            href="/dashboard"
            className="inline-block bg-blue-600 hover:bg-blue-700 text-white px-8 py-3 rounded-lg font-semibold transition"
          >
            Create Free Account →
          </Link>
        </div>
      </div>
    </div>
  )
}
