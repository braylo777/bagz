import { useEffect } from 'react'
import type { ReactNode } from 'react'
import { motion, useReducedMotion } from 'framer-motion'
import type { Route } from '../routes.ts'
import { go } from '../routes.ts'
import { useBagz } from '../store.tsx'
import { IconChat, IconDeck, IconMe } from './Icons.tsx'

const TITLES: Record<Route, string> = {
  splash: 'BAGZ',
  bagz: 'BAGZ',
  chat: 'BAGZ · Chat',
  profile: 'BAGZ · Profile',
  checkout: 'BAGZ · Checkout',
}

const TABS: { route: Route; label: string; icon: () => ReactNode }[] = [
  { route: 'bagz', label: 'Bagz', icon: IconDeck },
  { route: 'chat', label: 'Chat', icon: IconChat },
  { route: 'profile', label: 'Profile', icon: IconMe },
]

export function Shell({ route, children }: { route: Route; children: ReactNode }) {
  const showTabs = route !== 'splash' && route !== 'checkout'
  const { bags } = useBagz()
  const reduce = useReducedMotion()

  useEffect(() => {
    document.title = TITLES[route]
  }, [route])

  return (
    <div className="stage">
      <div className="device">
        <div className="bezel">
          <div className="island" aria-hidden="true" />
          <div className="glass">
            <div className="clock" aria-hidden="true">
              <span>9:41</span>
              <span className="battery" />
            </div>
            <motion.div
              className="glass-body"
              key={route}
              initial={reduce ? false : { opacity: 0, y: 18 }}
              animate={{ opacity: 1, y: 0 }}
              transition={reduce ? { duration: 0 } : { type: 'spring', stiffness: 460, damping: 34 }}
            >
              {children}
            </motion.div>
            {showTabs ? (
              <nav className="tabs" aria-label="Primary">
                {TABS.map((tab) => {
                  const Icon = tab.icon
                  const current = route === tab.route
                  return (
                    <button
                      key={tab.route}
                      type="button"
                      className="tab"
                      aria-current={current ? 'page' : undefined}
                      onClick={() => go(tab.route)}
                    >
                      <Icon />
                      <span>{tab.label}</span>
                      {tab.route === 'bagz' && bags.length > 0 ? (
                        <span className="badge">{bags.length > 9 ? '9+' : bags.length}</span>
                      ) : null}
                    </button>
                  )
                })}
              </nav>
            ) : null}
            <div className="home" aria-hidden="true" />
          </div>
        </div>
      </div>
      <BrandCredit />
    </div>
  )
}

/* Braylo SI built-by credit. Sits on the stage outside the phone frame; desktop only (see .bsi-credit in index.css). */
function BrandCredit() {
  return (
    <a className="bsi-credit" href="https://braylo.si" aria-label="Built by Brandon Lopez, Braylo SI">
      <img className="bsi-credit-hs" src="/brand/braylo-headshot.jpg" width={44} height={44} alt="" />
      <span className="bsi-credit-text">
        <img className="bsi-credit-sig" src="/brand/braylo-signature.webp" width={93} height={32} alt="" />
        <span>Built by Brandon Lopez, Founder · braylo.si</span>
      </span>
    </a>
  )
}
