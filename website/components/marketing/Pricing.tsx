import Link from 'next/link'

export function Pricing() {
  const plans = [
    {
      name: 'Free',
      price: '$0',
      description: 'Perfect for getting started',
      features: [
        '1M requests/month',
        '128MB memory',
        '3 languages (Python, Node.js, Java)',
        'Community support',
        'Public projects only',
      ],
      cta: 'Start Free',
      href: '/dashboard',
      popular: false,
    },
    {
      name: 'Pro',
      price: '$29',
      description: 'For professional developers',
      features: [
        '10M requests/month',
        'Up to 2GB memory',
        'All languages',
        'Priority support',
        'Private projects',
        'Custom domains',
        'Team collaboration',
      ],
      cta: 'Start Pro',
      href: '/dashboard?plan=pro',
      popular: true,
    },
    {
      name: 'Enterprise',
      price: 'Custom',
      description: 'For teams and organizations',
      features: [
        'Unlimited requests',
        'Dedicated resources',
        'SLA guarantees',
        '24/7 support',
        'Self-hosted option',
        'Custom integrations',
        'Training & onboarding',
      ],
      cta: 'Contact Sales',
      href: '/contact',
      popular: false,
    },
  ]

  return (
    <section className="py-20 px-4 sm:px-6 lg:px-8 bg-black">
      <div className="max-w-7xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-4xl md:text-5xl font-bold text-white mb-4">
            Simple, Transparent Pricing
          </h2>
          <p className="text-xl text-gray-400">
            No hidden fees. No surprise charges. Cancel anytime.
          </p>
        </div>

        <div className="grid md:grid-cols-3 gap-8 max-w-6xl mx-auto">
          {plans.map((plan) => (
            <div
              key={plan.name}
              className={`relative bg-gray-900 rounded-2xl p-8 border-2 transition ${
                plan.popular
                  ? 'border-blue-500 shadow-lg shadow-blue-500/20'
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
                <div className="mb-4">
                  <span className="text-5xl font-bold text-white">{plan.price}</span>
                  {plan.price !== 'Custom' && (
                    <span className="text-gray-400">/month</span>
                  )}
                </div>
                <p className="text-gray-400">{plan.description}</p>
              </div>

              <ul className="space-y-4 mb-8">
                {plan.features.map((feature) => (
                  <li key={feature} className="flex items-start">
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
                    <span className="text-gray-300">{feature}</span>
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

        <div className="mt-16 text-center">
          <p className="text-gray-400 mb-8">
            All plans include 0ms cold starts, multi-language support, and access to our API
          </p>
          <Link
            href="/pricing"
            className="text-blue-400 hover:text-blue-300 font-semibold"
          >
            Compare all features →
          </Link>
        </div>
      </div>
    </section>
  )
}
