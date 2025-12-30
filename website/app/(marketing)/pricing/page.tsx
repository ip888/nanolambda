import { Header } from '@/components/marketing/Header'
import { Footer } from '@/components/marketing/Footer'
import Link from 'next/link'

export default function PricingPage() {
  const plans = [
    {
      name: 'Free',
      price: '$0',
      period: 'forever',
      description: 'Perfect for hobbyists and testing',
      features: [
        { name: '1M requests per month', included: true },
        { name: '128MB memory limit', included: true },
        { name: '30 second timeout', included: true },
        { name: 'Python, Node.js, Java support', included: true },
        { name: '0ms warm starts', included: true },
        { name: 'Community support', included: true },
        { name: 'Public projects only', included: true },
        { name: 'Basic metrics', included: true },
        { name: 'Custom domains', included: false },
        { name: 'Priority support', included: false },
        { name: 'Team collaboration', included: false },
        { name: 'SLA guarantee', included: false },
      ],
      cta: 'Start Free',
      href: '/dashboard',
      popular: false,
    },
    {
      name: 'Pro',
      price: '$29',
      period: 'per month',
      description: 'For professional developers and startups',
      features: [
        { name: '10M requests per month', included: true },
        { name: 'Up to 2GB memory', included: true },
        { name: '5 minute timeout', included: true },
        { name: 'All language runtimes', included: true },
        { name: '0ms warm starts', included: true },
        { name: 'Priority email support (24h response)', included: true },
        { name: 'Private projects', included: true },
        { name: 'Advanced metrics & logs', included: true },
        { name: 'Custom domains', included: true },
        { name: 'Team collaboration (5 users)', included: true },
        { name: 'Git integration', included: true },
        { name: '99.9% uptime SLA', included: true },
      ],
      cta: 'Start Pro Trial',
      href: '/dashboard?plan=pro',
      popular: true,
    },
    {
      name: 'Enterprise',
      price: 'Custom',
      period: 'contact us',
      description: 'For teams and large-scale applications',
      features: [
        { name: 'Unlimited requests', included: true },
        { name: 'Custom memory limits', included: true },
        { name: 'Custom timeouts', included: true },
        { name: 'Dedicated support engineer', included: true },
        { name: 'Self-hosted option', included: true },
        { name: '24/7 phone & chat support', included: true },
        { name: 'Unlimited private projects', included: true },
        { name: 'Custom integrations', included: true },
        { name: 'White-label option', included: true },
        { name: 'Unlimited team members', included: true },
        { name: 'Training & onboarding', included: true },
        { name: '99.99% uptime SLA', included: true },
      ],
      cta: 'Contact Sales',
      href: '/contact',
      popular: false,
    },
  ]

  const faqs = [
    {
      question: 'What happens if I exceed my request limit?',
      answer: 'On the Free plan, requests will be rate-limited once you hit the limit. On Pro and Enterprise, we\'ll notify you and you can upgrade or pay for overage at $1 per million additional requests.',
    },
    {
      question: 'Can I change plans at any time?',
      answer: 'Yes! You can upgrade or downgrade your plan at any time. Upgrades take effect immediately, and downgrades at the end of your billing cycle.',
    },
    {
      question: 'What payment methods do you accept?',
      answer: 'We accept all major credit cards (Visa, Mastercard, Amex) and can arrange invoice billing for Enterprise customers.',
    },
    {
      question: 'Is there a free trial for Pro?',
      answer: 'Yes! Get 14 days of Pro features free when you sign up. No credit card required.',
    },
    {
      question: 'What\'s included in "0ms warm starts"?',
      answer: 'NanoLambda uses process pooling to keep your functions warm. After the first execution (cold start), subsequent requests execute in 0-2 milliseconds - faster than any competitor.',
    },
    {
      question: 'Can I self-host NanoLambda?',
      answer: 'Yes! NanoLambda is open source (MIT license). You can self-host on any infrastructure. Enterprise customers get dedicated support for self-hosted deployments.',
    },
    {
      question: 'What regions are supported?',
      answer: 'Currently US and EU. Enterprise customers can request deployment in additional regions or use self-hosting for any location.',
    },
    {
      question: 'Do you offer discounts for nonprofits or education?',
      answer: 'Yes! We offer 50% off Pro plans for verified nonprofits and educational institutions. Contact us with your organization details.',
    },
  ]

  return (
    <>
      <Header />
      
      <main className="pt-20 bg-black">
        {/* Hero */}
        <section className="py-20 px-4 sm:px-6 lg:px-8">
          <div className="max-w-4xl mx-auto text-center">
            <h1 className="text-5xl md:text-6xl font-bold text-white mb-6">
              Simple, Transparent Pricing
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              No hidden fees. No surprise charges. Pay only for what you use.
            </p>
            <div className="flex items-center justify-center space-x-6 text-gray-400 text-sm">
              <div className="flex items-center">
                <svg className="w-5 h-5 text-green-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                </svg>
                14-day free trial
              </div>
              <div className="flex items-center">
                <svg className="w-5 h-5 text-green-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                </svg>
                Cancel anytime
              </div>
              <div className="flex items-center">
                <svg className="w-5 h-5 text-green-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                </svg>
                No credit card required
              </div>
            </div>
          </div>
        </section>

        {/* Pricing cards */}
        <section className="py-20 px-4 sm:px-6 lg:px-8 bg-gray-950">
          <div className="max-w-7xl mx-auto">
            <div className="grid md:grid-cols-3 gap-8">
              {plans.map((plan) => (
                <div
                  key={plan.name}
                  className={`relative bg-gray-900 rounded-2xl p-8 border-2 transition ${
                    plan.popular
                      ? 'border-blue-500 shadow-xl shadow-blue-500/20 scale-105'
                      : 'border-gray-800 hover:border-gray-700'
                  }`}
                >
                  {plan.popular && (
                    <div className="absolute -top-4 left-1/2 transform -translate-x-1/2">
                      <span className="bg-blue-600 text-white px-4 py-1 rounded-full text-sm font-semibold">
                        Most Popular
                      </span>
                    </div>
                  )}

                  <div className="text-center mb-8">
                    <h3 className="text-2xl font-bold text-white mb-2">{plan.name}</h3>
                    <div className="mb-2">
                      <span className="text-5xl font-bold text-white">{plan.price}</span>
                      {plan.price !== 'Custom' && plan.price !== '$0' && (
                        <span className="text-gray-400 text-lg">/mo</span>
                      )}
                    </div>
                    <p className="text-gray-400 text-sm">{plan.description}</p>
                  </div>

                  <ul className="space-y-3 mb-8">
                    {plan.features.map((feature) => (
                      <li key={feature.name} className="flex items-start">
                        {feature.included ? (
                          <svg
                            className="w-5 h-5 text-green-400 mr-3 mt-0.5 flex-shrink-0"
                            fill="currentColor"
                            viewBox="0 0 20 20"
                          >
                            <path
                              fillRule="evenodd"
                              d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                              clipRule="evenodd"
                            />
                          </svg>
                        ) : (
                          <svg
                            className="w-5 h-5 text-gray-600 mr-3 mt-0.5 flex-shrink-0"
                            fill="currentColor"
                            viewBox="0 0 20 20"
                          >
                            <path
                              fillRule="evenodd"
                              d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                              clipRule="evenodd"
                            />
                          </svg>
                        )}
                        <span className={feature.included ? 'text-gray-300' : 'text-gray-600'}>
                          {feature.name}
                        </span>
                      </li>
                    ))}
                  </ul>

                  <Link
                    href={plan.href}
                    className={`block w-full text-center px-6 py-3 rounded-lg font-semibold transition ${
                      plan.popular
                        ? 'bg-blue-600 hover:bg-blue-700 text-white'
                        : 'bg-gray-800 hover:bg-gray-700 text-white'
                    }`}
                  >
                    {plan.cta}
                  </Link>
                </div>
              ))}
            </div>

            <div className="mt-12 text-center text-gray-400">
              <p className="mb-4">All plans include 0ms warm starts, multi-language support, and real-time metrics</p>
              <p className="text-sm">
                Need more? <Link href="/contact" className="text-blue-400 hover:text-blue-300">Contact us</Link> for custom enterprise solutions
              </p>
            </div>
          </div>
        </section>

        {/* FAQs */}
        <section className="py-20 px-4 sm:px-6 lg:px-8">
          <div className="max-w-4xl mx-auto">
            <h2 className="text-4xl font-bold text-white text-center mb-12">
              Frequently Asked Questions
            </h2>
            <div className="space-y-6">
              {faqs.map((faq) => (
                <details
                  key={faq.question}
                  className="bg-gray-900 border border-gray-800 rounded-lg p-6 group"
                >
                  <summary className="cursor-pointer text-lg font-semibold text-white flex items-center justify-between">
                    {faq.question}
                    <svg
                      className="w-5 h-5 text-gray-400 group-open:rotate-180 transition"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                    </svg>
                  </summary>
                  <p className="mt-4 text-gray-400 leading-relaxed">{faq.answer}</p>
                </details>
              ))}
            </div>
          </div>
        </section>

        {/* CTA */}
        <section className="py-20 px-4 sm:px-6 lg:px-8 bg-gradient-to-br from-blue-900/20 via-purple-900/20 to-black">
          <div className="max-w-4xl mx-auto text-center">
            <h2 className="text-4xl font-bold text-white mb-6">
              Ready to Get Started?
            </h2>
            <p className="text-xl text-gray-300 mb-8">
              Start with the free plan. Upgrade when you're ready.
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Link
                href="/dashboard"
                className="bg-blue-600 hover:bg-blue-700 text-white px-10 py-4 rounded-lg text-lg font-semibold transition shadow-lg shadow-blue-500/50"
              >
                Start Free →
              </Link>
              <Link
                href="/docs"
                className="bg-gray-800 hover:bg-gray-700 text-white px-10 py-4 rounded-lg text-lg font-semibold transition border border-gray-700"
              >
                Read Docs
              </Link>
            </div>
          </div>
        </section>
      </main>

      <Footer />
    </>
  )
}
