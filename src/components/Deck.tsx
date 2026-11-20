import { useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent, Ref } from 'react'
import { jobs } from '../data/jobs.ts'
import { go } from '../routes.ts'
import { useBagz } from '../store.tsx'
import type { Job } from '../types.ts'
import { Mascot, Monogram, rarityOf, scoreOf, stickerFor } from './CardArt.tsx'
import { IconApply, IconPass } from './Icons.tsx'

const THRESHOLD = 104

export function Deck() {
  const { history, bag, superBag, pass, resetSwipes, bags, plan, xp, streak, combo, openCheckout } = useBagz()
  const decided = new Set(history.map((item) => item.id))
  const remaining = jobs.filter((job) => !decided.has(job.id))
  const top = remaining[0]
  const [announce, setAnnounce] = useState('')
  const [bagOpen, setBagOpen] = useState(false)
  const [flipped, setFlipped] = useState(false)
  const [toast, setToast] = useState<string | null>(null)
  const [fx, setFx] = useState<null | { kind: 'apply' | 'match' | 'liked' | 'super' | 'vip'; xp: number }>(null)
  const slotRef = useRef<HTMLDivElement | null>(null)
  const stampRef = useRef<HTMLSpanElement | null>(null)
  const passStampRef = useRef<HTMLSpanElement | null>(null)
  const streakRef = useRef<HTMLDivElement | null>(null)
  const drag = useRef({ active: false, pointerId: -1, startX: 0, startY: 0, dx: 0, dy: 0 })
  const leaving = useRef(false)
  const actions = useRef<{ fly: (decision: 'bag' | 'pass' | 'super') => void }>({
    fly: () => undefined,
  })

  function paint(x: number, y: number, animate: boolean) {
    const slot = slotRef.current
    if (!slot) return
    slot.style.transition = animate ? 'transform 460ms cubic-bezier(0.16, 1.45, 0.36, 1)' : 'none'
    const rot = Math.max(-18, Math.min(18, x / 16))
    slot.style.transform = `translate3d(${x}px, ${y * 0.25}px, 0) rotate(${rot}deg)`
    const apply = Math.max(0, Math.min(1, x / THRESHOLD))
    const passAmt = Math.max(0, Math.min(1, -x / THRESHOLD))
    const mag = Math.max(apply, passAmt)
    if (stampRef.current) {
      stampRef.current.style.opacity = String(apply)
      stampRef.current.style.transform = `rotate(-14deg) scale(${0.8 + apply * 0.28})`
    }
    if (passStampRef.current) {
      passStampRef.current.style.opacity = String(passAmt)
      passStampRef.current.style.transform = `rotate(12deg) scale(${0.8 + passAmt * 0.28})`
    }
    if (streakRef.current) {
      streakRef.current.style.opacity = String(mag)
      streakRef.current.classList.toggle('trail-right', x < 0)
      const bars = streakRef.current.querySelectorAll('i')
      const widths = [36, 22, 30]
      bars.forEach((bar, index) => {
        const base = widths[index] ?? 24
        bar.style.width = `${Math.round(base * (0.35 + mag * 0.65))}px`
      })
    }
  }

  function buzz(ms: number) {
    if (typeof navigator !== 'undefined' && typeof navigator.vibrate === 'function') navigator.vibrate(ms)
  }

  function fly(decision: 'bag' | 'pass' | 'super') {
    if (!top || leaving.current) return
    leaving.current = true
    const job = top
    const gain = decision === 'super' ? 25 : 10
    setAnnounce(decision === 'pass' ? `Passed ${job.title}` : `Applied ${job.title}`)
    paint(decision === 'pass' ? -720 : 720, decision === 'pass' ? 20 : -10, true)
    if (decision !== 'pass') {
      const jackpot = job.matchPct >= 92 || Math.random() < 0.35
      const liked = jackpot && Math.random() < 0.5
      setFx({
        kind: decision === 'super' ? 'super' : liked ? 'liked' : jackpot ? 'match' : 'apply',
        xp: gain,
      })
      if (Math.random() < 0.4) {
        setToast(`${job.company} viewed you`)
        window.setTimeout(() => setToast(null), 1600)
      }
      buzz(decision === 'super' ? 28 : 16)
      window.setTimeout(() => setFx(null), 780)
    } else buzz(8)
    window.setTimeout(() => {
      if (decision === 'pass') pass(job.id)
      else if (decision === 'super') superBag(job.id)
      else bag(job.id)
      leaving.current = false
      setFlipped(false)
    }, 320)
  }

  useEffect(() => {
    const show = window.setTimeout(() => setToast('3 new bags in Austin'), 900)
    const hide = window.setTimeout(() => setToast(null), 2800)
    const onTilt = (event: DeviceOrientationEvent) => {
      const x = 50 + (event.gamma ?? 0)
      const y = 40 + (event.beta ?? 0) / 3
      document.documentElement.style.setProperty('--holo-x', `${x}%`)
      document.documentElement.style.setProperty('--holo-y', `${y}%`)
    }
    window.addEventListener('deviceorientation', onTilt)
    return () => {
      window.clearTimeout(show)
      window.clearTimeout(hide)
      window.removeEventListener('deviceorientation', onTilt)
    }
  }, [])

  useEffect(() => {
    actions.current.fly = fly
  })

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === 'ArrowRight') {
        event.preventDefault()
        actions.current.fly('bag')
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault()
        actions.current.fly('pass')
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  function onPointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (!top || leaving.current || !event.isPrimary) return
    drag.current = { active: true, pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, dx: 0, dy: 0 }
    event.currentTarget.setPointerCapture(event.pointerId)
    paint(0, 0, false)
  }

  function onPointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    if (!drag.current.active || event.pointerId !== drag.current.pointerId) return
    const dx = event.clientX - drag.current.startX
    const dy = event.clientY - drag.current.startY
    drag.current.dx = dx
    drag.current.dy = dy
    const rect = event.currentTarget.getBoundingClientRect()
    event.currentTarget.style.setProperty('--holo-x', `${((event.clientX - rect.left) / rect.width) * 100}%`)
    event.currentTarget.style.setProperty('--holo-y', `${((event.clientY - rect.top) / rect.height) * 100}%`)
    if (dy < -24 && Math.abs(dy) > Math.abs(dx)) {
      paint(dx * 0.15, dy, false)
      return
    }
    paint(dx, dy, false)
  }

  function onPointerUp(event: ReactPointerEvent<HTMLDivElement>) {
    if (!drag.current.active || event.pointerId !== drag.current.pointerId) return
    const dx = drag.current.dx
    const dy = drag.current.dy
    drag.current.active = false
    const tap = Math.abs(dx) < 14 && Math.abs(dy) < 14
    const swipeUp = dy < -64 && Math.abs(dx) < 70
    if (tap) {
      setFlipped((open) => !open)
      paint(0, 0, true)
      return
    }
    if (swipeUp) {
      if (plan === 'vip') fly('super')
      else {
        setFx({ kind: 'vip', xp: 0 })
        buzz(10)
        window.setTimeout(() => {
          setFx(null)
          openCheckout('vip')
        }, 520)
        paint(0, 0, true)
      }
      return
    }
    if (dx >= THRESHOLD) fly('bag')
    else if (dx <= -THRESHOLD) fly('pass')
    else paint(0, 0, true)
  }

  const stack = remaining.slice(0, 3)
  const painted = [...stack].reverse()
  const chased = bags.reduce((sum, id) => sum + (jobs.find((job) => job.id === id)?.chase ?? 0), 0)
  const watchers = jobs.filter((job) => !decided.has(job.id)).slice(-3)
  const rank = xp >= 120 ? 'Bag Secured' : xp >= 40 ? 'Closer' : 'Rookie'

  return (
    <section className={`screen deck ${fx ? 'hot' : ''}`} data-screen="bagz">
      <div className="hud" aria-label="Streak and level">
        <span className="streak-n">{streak}</span>
        <div className="xp-track" aria-hidden="true">
          <span style={{ width: `${((xp % 40) / 40) * 100}%` }} />
        </div>
        <span className={combo > 1 ? 'combo on' : 'combo'}>{combo > 1 ? `x${combo}` : rank}</span>
      </div>
      <div className="loop-row">
        <span>{remaining.length} · drop</span>
        <button type="button" className={plan === 'vip' ? 'eyes clear' : 'eyes'} onClick={() => { if (plan !== 'vip') openCheckout('vip') }}>
          {watchers.map((job) => (
            <span key={job.id}>{job.company}</span>
          ))}
        </button>
        <span>${chased.toLocaleString('en-US')}</span>
      </div>
      {top ? (
        <>
          <div className="stack">
            {painted.map((job) => {
              const depth = stack.findIndex((item) => item.id === job.id)
              const isTop = depth === 0
              return (
                <div
                  key={job.id}
                  className={`slot ${isTop ? 'top' : `back-${depth}`}`}
                  ref={isTop ? slotRef : undefined}
                  onPointerDown={isTop ? onPointerDown : undefined}
                  onPointerMove={isTop ? onPointerMove : undefined}
                  onPointerUp={isTop ? onPointerUp : undefined}
                  onPointerCancel={isTop ? onPointerUp : undefined}
                >
                  {isTop ? (
                    <>
                      <div ref={streakRef} className="streaks" aria-hidden="true">
                        <i />
                        <i />
                        <i />
                      </div>
                      <span ref={passStampRef} className="stamp stamp-pass">
                        Pass
                      </span>
                    </>
                  ) : null}
                  <div className={`flipper ${isTop && flipped ? 'flipped' : ''}`}>
                    <CardFace job={job} isTop={isTop} stampRef={isTop ? stampRef : undefined} />
                    <CardBack job={job} />
                  </div>
                </div>
              )
            })}
          </div>
          <div className="actions">
            <button type="button" className="round" onClick={() => fly('pass')} aria-label={`Pass on ${top.title}`}>
              <IconPass />
            </button>
            <button type="button" className="round apply" onClick={() => fly('bag')} aria-label={`Apply to ${top.title}`}>
              <IconApply />
            </button>
          </div>
          <div className="deck-tools">
            <span className="caption">{plan === 'vip' ? 'Up = super' : 'Up = VIP'}</span>
            <button type="button" className="text-btn" onClick={() => setBagOpen(true)}>
              {bags.length}
            </button>
          </div>
        </>
      ) : (
        <div className="empty">
          <h2 className="display display-s">
            <span>Clear.</span>
          </h2>
          <button type="button" className="btn-acid" onClick={() => go('chat')}>
            Chat
          </button>
          <button type="button" className="btn-line" onClick={resetSwipes}>
            Replay deck
          </button>
        </div>
      )}
      {fx ? (
        <div className={`burst ${fx.kind}`} aria-live="polite">
          <div className="pulse" />
          <div className="bits" aria-hidden="true">
            <i /><i /><i /><i /><i /><i /><i /><i />
          </div>
          <p className="display">
            {fx.kind === 'match' ? 'Matched' : fx.kind === 'liked' ? 'Liked you' : fx.kind === 'super' ? 'Super' : fx.kind === 'vip' ? 'VIP' : `+${fx.xp}`}
          </p>
        </div>
      ) : null}
      <p className="sr-only" aria-live="polite">
        {announce}
      </p>
      {toast ? <p className="toast">{toast}</p> : null}
      {bagOpen ? (
        <div className="sheet-wrap">
          <button type="button" className="sheet-backdrop" aria-label="Close the bag" onClick={() => setBagOpen(false)} />
          <div className="sheet" role="dialog" aria-modal="true" aria-labelledby="bag-title">
            <h2 id="bag-title" className="display display-s">
              The <span>bag.</span>
            </h2>
            {bags.length === 0 ? (
              <p className="body">Nothing here yet. Swipe right to apply.</p>
            ) : (
              <ul className="bag-list">
                {bags.map((id) => {
                  const job = jobs.find((item) => item.id === id)
                  if (!job) return null
                  return (
                    <li key={job.id}>
                      <strong>{job.title}</strong>
                      <em>
                        {job.company} · {job.pay}
                      </em>
                    </li>
                  )
                })}
              </ul>
            )}
            <button type="button" className="btn-line" onClick={() => setBagOpen(false)}>
              Close
            </button>
          </div>
        </div>
      ) : null}
    </section>
  )
}

function CardFace({
  job,
  isTop,
  stampRef,
}: {
  job: Job
  isTop: boolean
  stampRef?: Ref<HTMLSpanElement>
}) {
  const rarity = rarityOf(job.matchPct)
  const sticker = stickerFor(job)
  const score = scoreOf(job)
  return (
    <article
      className={`card face ${rarity}`}
      data-job-id={isTop ? job.id : undefined}
      aria-hidden={isTop ? undefined : true}
      aria-label={isTop ? `${job.title} at ${job.company}, ${job.pay}` : undefined}
    >
      <div className="wash" aria-hidden="true">
        <Monogram letter={job.company.slice(0, 1)} />
        <Mascot id={job.id} />
      </div>
      {sticker ? <span className="sticker">{sticker}</span> : null}
      <span className="score">{score}</span>
      <div className="recruiter">
        <i>{job.company.slice(0, 1)}</i>
        <span>{job.reply.replace('Replies in ', '').replace('Replies ', '')}</span>
      </div>
      {isTop ? (
        <span ref={stampRef} className="stamp">
          Apply
        </span>
      ) : null}
      <h2 className="job-title">{job.title}</h2>
      <p className="company">{job.company}</p>
      <p className="pay">{job.pay}</p>
      <div className="bars">
        <Bar label="Pay" value={job.payScore} />
        <Bar label="Fit" value={job.matchPct} />
        <Bar label="Growth" value={job.growth} />
        <Bar label="Vibe" value={job.vibe} />
      </div>
    </article>
  )
}

function CardBack({ job }: { job: Job }) {
  return (
    <article className={`card backface ${rarityOf(job.matchPct)}`}>
      <p className="pay detail-pay">{job.pay}</p>
      <p className="kicker">{job.mode} · {job.area}</p>
      <h2 className="job-title back-title">{job.title}</h2>
      <ul className="reasons">
        {job.reasons.slice(0, 2).map((reason) => (
          <li key={reason}>{reason}</li>
        ))}
      </ul>
      <div className="chips">
        {job.skills.map((skill) => (
          <span key={skill}>{skill}</span>
        ))}
      </div>
      <div className="chips">
        {job.perks.map((perk) => (
          <span key={perk}>{perk}</span>
        ))}
      </div>
      <p className="meta">{job.size} · {job.stage} · {job.posted} · {job.applicants}</p>
    </article>
  )
}

function Bar({ label, value }: { label: string; value: number }) {
  return (
    <div className="bar">
      <span>{label}</span>
      <i><b style={{ width: `${value}%` }} /></i>
    </div>
  )
}
