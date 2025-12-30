import fs from 'fs'
import path from 'path'
import matter from 'gray-matter'
import { MDXRemote } from 'next-mdx-remote/rsc'
import Link from 'next/link'

// Custom components for MDX
const components = {
  h1: (props: any) => <h1 className="text-4xl font-bold text-white mb-6 mt-8" {...props} />,
  h2: (props: any) => <h2 className="text-3xl font-bold text-white mb-4 mt-8 pb-2 border-b border-gray-800" {...props} />,
  h3: (props: any) => <h3 className="text-2xl font-bold text-white mb-3 mt-6" {...props} />,
  h4: (props: any) => <h4 className="text-xl font-bold text-white mb-2 mt-4" {...props} />,
  p: (props: any) => <p className="text-gray-300 mb-4 leading-relaxed" {...props} />,
  a: (props: any) => <Link href={props.href} className="text-blue-400 hover:text-blue-300 underline" {...props} />,
  code: (props: any) => {
    if (props.className) {
      // Block code
      return (
        <code className="block bg-gray-900 text-gray-300 p-4 rounded-lg overflow-x-auto font-mono text-sm my-4" {...props} />
      )
    }
    // Inline code
    return <code className="bg-gray-800 text-blue-400 px-2 py-1 rounded font-mono text-sm" {...props} />
  },
  pre: (props: any) => <pre className="bg-gray-900 p-4 rounded-lg overflow-x-auto my-4" {...props} />,
  ul: (props: any) => <ul className="list-disc list-inside text-gray-300 mb-4 space-y-2" {...props} />,
  ol: (props: any) => <ol className="list-decimal list-inside text-gray-300 mb-4 space-y-2" {...props} />,
  li: (props: any) => <li className="text-gray-300" {...props} />,
  blockquote: (props: any) => (
    <blockquote className="border-l-4 border-blue-500 pl-4 italic text-gray-400 my-4" {...props} />
  ),
  table: (props: any) => (
    <div className="overflow-x-auto my-4">
      <table className="min-w-full border border-gray-800" {...props} />
    </div>
  ),
  th: (props: any) => <th className="bg-gray-900 text-white px-4 py-2 text-left border-b border-gray-800" {...props} />,
  td: (props: any) => <td className="text-gray-300 px-4 py-2 border-b border-gray-800" {...props} />,
  hr: () => <hr className="border-gray-800 my-8" />,
}

function getDocContent(slug: string) {
  try {
    const filePath = path.join(process.cwd(), 'content', 'docs', `${slug}.mdx`)
    const fileContent = fs.readFileSync(filePath, 'utf8')
    const { data, content } = matter(fileContent)
    return { frontmatter: data, content }
  } catch (error) {
    return null
  }
}

export default async function DocPage({
  params,
}: {
  params: Promise<{ slug?: string[] }>
}) {
  const resolvedParams = await params
  const slug = resolvedParams.slug ? resolvedParams.slug.join('/') : 'getting-started'
  const doc = getDocContent(slug)

  if (!doc) {
    return (
      <div className="p-8 max-w-4xl">
        <h1 className="text-4xl font-bold text-white mb-4">Page Not Found</h1>
        <p className="text-gray-400 mb-6">
          The documentation page you're looking for doesn't exist yet.
        </p>
        <Link
          href="/docs/getting-started"
          className="text-blue-400 hover:text-blue-300"
        >
          ← Back to Getting Started
        </Link>
      </div>
    )
  }

  return (
    <article className="p-8 max-w-4xl">
      {/* Header */}
      <header className="mb-8 pb-8 border-b border-gray-800">
        <h1 className="text-4xl font-bold text-white mb-2">{doc.frontmatter.title}</h1>
        {doc.frontmatter.description && (
          <p className="text-xl text-gray-400">{doc.frontmatter.description}</p>
        )}
      </header>

      {/* Content */}
      <div className="prose prose-invert max-w-none">
        <MDXRemote source={doc.content} components={components} />
      </div>

      {/* Footer */}
      <footer className="mt-12 pt-8 border-t border-gray-800">
        <div className="flex items-center justify-between">
          <Link
            href="/docs"
            className="text-blue-400 hover:text-blue-300 flex items-center"
          >
            <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
            Back to Docs
          </Link>

          <a
            href={`https://github.com/ip888/nanolambda/edit/main/website/content/docs/${slug}.mdx`}
            target="_blank"
            rel="noopener noreferrer"
            className="text-gray-400 hover:text-white flex items-center text-sm"
          >
            Edit on GitHub
            <svg className="w-4 h-4 ml-2" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
            </svg>
          </a>
        </div>
      </footer>
    </article>
  )
}
