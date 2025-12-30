import { Hero } from '@/components/marketing/Hero'
import { SpeedComparison } from '@/components/marketing/SpeedComparison'
import { CodeExample } from '@/components/marketing/CodeExample'
import { Features } from '@/components/marketing/Features'
import { Pricing } from '@/components/marketing/Pricing'
import { CTA } from '@/components/marketing/CTA'

export default function Home() {
  return (
    <>
      <Hero />
      <SpeedComparison />
      <CodeExample />
      <Features />
      <Pricing />
      <CTA />
    </>
  )
}
