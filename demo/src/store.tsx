import { createContext, useContext, useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { jobs } from './data/jobs.ts'
import { go } from './routes.ts'
import type { DecisionName, DecisionRecord, Plan } from './types.ts'

const STORAGE_KEY = 'bagz-demo-v2'

type Persisted = {
  plan: Plan
  history: DecisionRecord[]
  xp: number
  streak: number
  lastDay: string
}

type BagzStore = {
  plan: Plan
  history: DecisionRecord[]
  bags: string[]
  passes: string[]
  xp: number
  streak: number
  combo: number
  offer: 'pro' | 'vip'
  receipt: boolean
  bag: (id: string) => void
  superBag: (id: string) => void
  pass: (id: string) => void
  undo: () => void
  resetSwipes: () => void
  openCheckout: (plan: 'pro' | 'vip') => void
  confirmCheckout: () => void
}

const BagzContext = createContext<BagzStore | null>(null)

function today() {
  return new Date().toISOString().slice(0, 10)
}

function score(current: Persisted, gain: number): Persisted {
  const day = today()
  return {
    ...current,
    xp: current.xp + gain,
    streak: current.lastDay === day ? current.streak : current.streak + 1,
    lastDay: day,
  }
}

function sanitize(value: unknown): Persisted {
  const empty: Persisted = { plan: 'free', history: [], xp: 0, streak: 0, lastDay: '' }
  if (!value || typeof value !== 'object') return empty
  const record = value as { plan?: unknown; history?: unknown }
  const plan: Plan = record.plan === 'pro' || record.plan === 'vip' || record.plan === 'agent' ? (record.plan === 'agent' ? 'vip' : record.plan) : 'free'
  const ids = new Set(jobs.map((job) => job.id))
  const history: DecisionRecord[] = []
  const seen = new Set<string>()
  if (Array.isArray(record.history)) {
    for (const item of record.history) {
      if (!item || typeof item !== 'object') continue
      const row = item as { id?: unknown; decision?: unknown }
      if (typeof row.id !== 'string' || !ids.has(row.id) || seen.has(row.id)) continue
      if (row.decision !== 'bag' && row.decision !== 'pass') continue
      seen.add(row.id)
      history.push({ id: row.id, decision: row.decision })
    }
  }
  const xp = typeof (record as { xp?: unknown }).xp === 'number' ? (record as { xp: number }).xp : 0
  const streak = typeof (record as { streak?: unknown }).streak === 'number' ? (record as { streak: number }).streak : 0
  const lastDay = typeof (record as { lastDay?: unknown }).lastDay === 'string' ? (record as { lastDay: string }).lastDay : ''
  return { plan, history, xp, streak, lastDay }
}

function loadPersisted(): Persisted {
  try {
    return sanitize(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null'))
  } catch {
    return sanitize(null)
  }
}

function decide(history: DecisionRecord[], id: string, decision: DecisionName): DecisionRecord[] {
  return [...history.filter((item) => item.id !== id), { id, decision }]
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [persisted, setPersisted] = useState<Persisted>(loadPersisted)
  const [offer, setOffer] = useState<'pro' | 'vip'>('vip')
  const [receipt, setReceipt] = useState(false)
  const [combo, setCombo] = useState(0)

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(persisted))
    } catch {
      // Demo still runs if storage is blocked.
    }
  }, [persisted])

  const bags = persisted.history
    .filter((item) => item.decision === 'bag')
    .map((item) => item.id)
    .reverse()
  const passes = persisted.history.filter((item) => item.decision === 'pass').map((item) => item.id)

  const value: BagzStore = {
    plan: persisted.plan,
    history: persisted.history,
    bags,
    passes,
    xp: persisted.xp,
    streak: persisted.streak,
    combo,
    offer,
    receipt,
    bag: (id) => {
      setCombo((current) => current + 1)
      setPersisted((current) => score({ ...current, history: decide(current.history, id, 'bag') }, 10))
    },
    superBag: (id) => {
      setCombo((current) => current + 1)
      setPersisted((current) => score({ ...current, history: decide(current.history, id, 'bag') }, 25))
    },
    pass: (id) => {
      setCombo(0)
      setPersisted((current) => ({ ...current, history: decide(current.history, id, 'pass') }))
    },
    undo: () => setPersisted((current) => ({ ...current, history: current.history.slice(0, -1) })),
    resetSwipes: () => setPersisted((current) => ({ ...current, history: [] })),
    openCheckout: (plan) => {
      setOffer(plan)
      setReceipt(false)
      go('checkout')
    },
    confirmCheckout: () => {
      setPersisted((current) => ({ ...current, plan: offer }))
      setReceipt(true)
    },
  }

  return <BagzContext.Provider value={value}>{children}</BagzContext.Provider>
}

export function useBagz(): BagzStore {
  const value = useContext(BagzContext)
  if (!value) throw new Error('useBagz must be used inside StoreProvider')
  return value
}
