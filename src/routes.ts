import { useEffect, useState } from 'react'

export type Route = 'splash' | 'bagz' | 'chat' | 'profile' | 'checkout'

const ALIASES: Record<string, Route> = {
  splash: 'splash',
  bagz: 'bagz',
  deck: 'bagz',
  chat: 'chat',
  agent: 'chat',
  profile: 'profile',
  tiers: 'profile',
  checkout: 'checkout',
}

export function parseRoute(hash: string): Route {
  const path = hash.replace(/^#\/?/, '')
  return ALIASES[path] ?? 'splash'
}

export function go(route: Route) {
  const next = `#/${route}`
  if (window.location.hash !== next) window.location.hash = next
}

export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parseRoute(window.location.hash))

  useEffect(() => {
    const onChange = () => setRoute(parseRoute(window.location.hash))
    window.addEventListener('hashchange', onChange)
    return () => window.removeEventListener('hashchange', onChange)
  }, [])

  return route
}
