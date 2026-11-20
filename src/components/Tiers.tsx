import { useBagz } from '../store.tsx'
import type { Plan } from '../types.ts'
import { IconCheck } from './Icons.tsx'

const TIERS: { id: Plan; name: string; price: string; cadence: string; points: string[] }[] = [
  {
    id: 'free',
    name: 'Free',
    price: '$0',
    cadence: '',
    points: ['Swipe to apply', 'AI-built profile from resume', 'Location / remote filters'],
  },
  {
    id: 'pro',
    name: 'Pro',
    price: '$2.99',
    cadence: '/month',
    points: ['Everything in Free', 'Boosted “Pro” badge', 'Ranked higher, in front of a human'],
  },
  {
    id: 'vip',
    name: 'VIP',
    price: '$8.99',
    cadence: '/month',
    points: ['Everything in Pro', 'AI agent auto-applies for you', 'Identity verification'],
  },
]

export function Tiers() {
  const { plan, openCheckout } = useBagz()

  return (
    <>
      <div className="scroll">
        <div className="tier-list">
          {TIERS.map((tier) => (
            <article key={tier.id} className={`tier ${tier.id} ${plan === tier.id ? 'current' : ''}`}>
              <div className="tier-head">
                <h2>
                  {tier.name}
                  {plan === tier.id ? <span className="now">Current</span> : null}
                </h2>
                <p className="tier-price">
                  {tier.price}
                  {tier.cadence ? <span>{tier.cadence}</span> : null}
                </p>
              </div>
              <ul>
                {tier.points.map((point) => (
                  <li key={point}>
                    <IconCheck size={15} />
                    {point}
                  </li>
                ))}
              </ul>
            </article>
          ))}
        </div>
      </div>
      <footer className="foot">
        <button type="button" className="btn-acid" onClick={() => openCheckout('vip')}>
          Get VIP →
        </button>
        <button type="button" className="text-btn" onClick={() => openCheckout('pro')}>
          Get Pro · $2.99
        </button>
      </footer>
    </>
  )
}
