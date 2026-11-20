export type Plan = 'free' | 'pro' | 'vip'

export type Job = {
  id: string
  title: string
  company: string
  pay: string
  payScore: number
  growth: number
  vibe: number
  chase: number
  area: string
  mode: string
  schedule: string
  posted: string
  applicants: string
  size: string
  stage: string
  matchPct: number
  reasons: string[]
  skills: string[]
  perks: string[]
  reply: string
  sends: string
}

export type DecisionName = 'bag' | 'pass'

export type DecisionRecord = {
  id: string
  decision: DecisionName
}
