import { motion, useReducedMotion } from 'framer-motion'
import { useEffect, useState } from 'react'
import { jobs } from '../data/jobs.ts'
import { useBagz } from '../store.tsx'
import type { Job } from '../types.ts'
import { IconBack } from './Icons.tsx'

const STAGGER = 700
const PEOPLE: Record<string, { name: string; note: string }> = {
  froth: { name: 'Sam Ruiz', note: 'Saturday works. East Austin, 8am.' },
  rack: { name: 'Jules Park', note: 'Come style the floor this weekend.' },
  ping: { name: 'Mina Chen', note: 'Saturday works. Downtown, 10am.' },
  poster: { name: 'Remy Cole', note: 'Friday review. Bring the posters.' },
  spoke: { name: 'Casey Nguyen', note: 'Start tomorrow. Bring your bike.' },
  latebird: { name: 'Luis Ortega', note: 'Thu–Sat door. You in?' },
  clip: { name: 'Ava Shah', note: 'Send two more reels before Friday.' },
  dayglow: { name: 'Noah Ellis', note: 'Mornings are open. Hyde Park.' },
  salsa: { name: 'Rio Alvarez', note: 'Trial Thursday night. Staff meal is on us.' },
  lace: { name: 'Priya Nand', note: 'Drop day is Saturday. You know sizes?' },
  quad: { name: 'Eden Brooks', note: 'Tabling Thursday on West Campus.' },
  carton: { name: 'Malik Adeyemi', note: 'First round in five days. Send GitHub.' },
}

type Phase = 'Queued' | 'Applying' | 'Applied' | 'Reply'

function queueFor(bagIds: string[]): Job[] {
  const ordered = [...bagIds, ...jobs.map((job) => job.id)]
  const seen = new Set<string>()
  const rows: Job[] = []
  for (const id of ordered) {
    if (seen.has(id)) continue
    const job = jobs.find((item) => item.id === id)
    if (!job) continue
    seen.add(id)
    rows.push(job)
    if (rows.length === 4) break
  }
  return rows
}

function phaseFor(elapsed: number, index: number): Phase {
  if (elapsed < 450) return 'Queued'
  if (elapsed < 1100) return 'Applying'
  if (index < 2 && elapsed >= 1900) return 'Reply'
  if (elapsed >= 1100) return 'Applied'
  return 'Applying'
}

function stamp(ms: number) {
  const total = 9 * 3600 + 41 * 60 + Math.max(0, Math.floor(ms / 1000))
  const minute = Math.floor(total / 60) % 60
  const second = total % 60
  const pad = (value: number) => String(value).padStart(2, '0')
  return `9:${pad(minute)}:${pad(second)}`
}

export function Chat() {
  const { bags, openCheckout } = useBagz()
  const rows = queueFor(bags)
  const reduceMotion = useReducedMotion()
  const reduce = Boolean(reduceMotion)
  const [ready, setReady] = useState(reduce)
  const [now, setNow] = useState(reduce ? 8000 : 0)
  const [openId, setOpenId] = useState<string | null>(null)
  const [sentNote, setSentNote] = useState(false)

  useEffect(() => {
    const wait = window.setTimeout(() => setReady(true), reduce ? 0 : 380)
    return () => window.clearTimeout(wait)
  }, [reduce])

  useEffect(() => {
    if (reduce) return
    const start = performance.now()
    const id = window.setInterval(() => setNow(performance.now() - start), 200)
    const stop = window.setTimeout(() => window.clearInterval(id), 8000)
    return () => {
      window.clearInterval(id)
      window.clearTimeout(stop)
    }
  }, [reduce])

  const phases = rows.map((_, index) => phaseFor(now - index * STAGGER, index))
  const sent = phases.filter((phase) => phase === 'Applied' || phase === 'Reply').length
  const replies = phases.filter((phase) => phase === 'Reply').length
  const hours = ((sent * 22) / 60).toFixed(1)
  const open = rows.find((job) => job.id === openId) ?? null
  const person = open ? PEOPLE[open.id] : undefined

  if (open && person) {
    return (
      <section className="screen chat" data-screen="chat">
        <button type="button" className="back" onClick={() => setOpenId(null)}>
          <IconBack />
          Chat
        </button>
        <header className="page-head">
          <p className="kicker">{open.company}</p>
          <h1 className="display display-m">
            {person.name.split(' ')[0]} <span>{person.name.split(' ')[1]}.</span>
          </h1>
        </header>
        <div className="thread">
          <p>
            <span>9:41</span>
            Your agent sent {open.sends.toLowerCase()}.
          </p>
          <p className="theirs">
            <span>9:43</span>
            {person.note}
          </p>
          {sentNote ? (
            <p className="mine">
              <span>9:44</span>
              I’ll be there.
            </p>
          ) : null}
        </div>
        <footer className="foot">
          <button type="button" className="btn-acid" onClick={() => setSentNote(true)} disabled={sentNote}>
            {sentNote ? 'Sent' : 'Send “I’ll be there”'}
          </button>
        </footer>
      </section>
    )
  }

  return (
    <section className="screen chat" data-screen="chat">
      <header className="page-head">
        <h1 className="display display-m">
          Your agent
          <br />
          <span>files it.</span>
        </h1>
      </header>
      <div className="payoff payoff-lg" aria-label="Results">
        <p>
          <b>{sent}</b>
          <span>sent</span>
        </p>
        <p>
          <b>{replies}</b>
          <span>{replies === 1 ? 'reply' : 'replies'}</span>
        </p>
        <p>
          <b>{hours}</b>
          <span>hrs saved</span>
        </p>
      </div>
      <div className="timeline">
        {!ready
          ? [0, 1, 2, 3].map((key) => <div key={key} className="shimmer" />)
          : rows.map((job, index) => {
          const elapsed = now - index * STAGGER
          const phase = phases[index] ?? 'Queued'
          const personRow = PEOPLE[job.id]
          const tappable = phase === 'Reply'
          const detail =
            phase === 'Reply'
              ? personRow?.note
              : phase === 'Applied'
                ? job.sends
                : phase === 'Applying'
                  ? 'Filing'
                  : `${job.mode} · ${job.area}`
          const inner = (
            <>
              <time>{stamp(Math.max(0, elapsed))}</time>
              <span>
                <strong>{job.title}</strong>
                <em>
                  {job.company} · {detail}
                </em>
              </span>
              <b className={`status ${phase.toLowerCase()}`}>{phase}</b>
            </>
          )
          const motionProps = {
            initial: reduce ? false : { opacity: 0, y: 18 },
            animate: { opacity: 1, y: 0 },
            transition: reduce ? { duration: 0 } : { type: 'spring' as const, stiffness: 480, damping: 32, delay: index * 0.05 },
          }
          if (!tappable) {
            return (
              <motion.div key={job.id} {...motionProps}>
                {inner}
              </motion.div>
            )
          }
          return (
            <motion.button key={job.id} type="button" onClick={() => setOpenId(job.id)} {...motionProps}>
              {inner}
            </motion.button>
          )
        })}
      </div>
      <footer className="foot">
        <button type="button" className="btn-acid" onClick={() => openCheckout('vip')}>
          Go VIP
        </button>
      </footer>
    </section>
  )
}
