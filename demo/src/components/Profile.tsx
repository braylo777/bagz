import { useState } from 'react'
import { useBagz } from '../store.tsx'
import type { Plan } from '../types.ts'

const PLAN_LABEL: Record<Plan, string> = {
  free: 'Free · $0',
  pro: 'Pro · $2.99/mo',
  vip: 'VIP · $8.99/mo',
}

const EXPERIENCE = [
  { role: 'Shift lead', company: 'Kindling Coffee', dates: 'Jun 2025 – Now', place: 'Austin' },
  { role: 'Door staff', company: 'Rainey nights', dates: 'Sep 2024 – May 2025', place: 'Austin' },
  { role: 'Campus ambassador', company: 'Student org', dates: 'Jan 2024 – Aug 2024', place: 'West Campus' },
]

const SKILLS = ['Espresso', 'Guest list', 'React', 'Group chat', 'Scheduling', 'Food handler']

export function Profile() {
  const { plan, openCheckout } = useBagz()
  const [resumeNote, setResumeNote] = useState('Alex-Rivera.pdf')

  return (
    <section className="screen profile" data-screen="profile">
      <div className="scroll profile-scroll">
        <div className="who">
          <div className="avatar" aria-hidden="true">
            AR
          </div>
          <div>
            <h1 className="display display-m">
              Alex <span>Rivera.</span>
            </h1>
            <p className="headline">Barista, host, and campus builder</p>
            <p className="meta">Austin, TX</p>
            <span className="chip-violet">Open to work</span>
          </div>
        </div>

        <div className="payoff" aria-label="Profile stats">
          <p>
            <b>6</b>
            <span>applied</span>
          </p>
          <p>
            <b>2</b>
            <span>replies</span>
          </p>
          <p>
            <b>1</b>
            <span>interview</span>
          </p>
        </div>

        <section>
          <p className="kicker">About</p>
          <p className="body">Cafe. Door. A desk that ships.</p>
        </section>

        <section>
          <p className="kicker">Experience</p>
          <ul className="xp">
            {EXPERIENCE.map((item) => (
              <li key={item.role}>
                <strong>{item.role}</strong>
                <em>
                  {item.company} · {item.place}
                </em>
                <span>{item.dates}</span>
              </li>
            ))}
          </ul>
        </section>

        <section>
          <p className="kicker">Skills</p>
          <div className="chips">
            {SKILLS.map((skill) => (
              <span key={skill}>{skill}</span>
            ))}
          </div>
        </section>

        <section>
          <p className="kicker">Education</p>
          <p className="xp-role">B.A. Communication</p>
          <p className="meta">UT Austin · expected 2027</p>
        </section>

        <section className="resume-card">
          <p className="kicker">Resume</p>
          <p className="xp-role">{resumeNote}</p>
          <div className="meter" aria-hidden="true">
            <span style={{ width: '78%' }} />
          </div>
          <p className="meta">Strength 78 · built from this file</p>
          <button
            type="button"
            className="btn-line"
            onClick={() => setResumeNote(resumeNote.endsWith('.pdf') ? 'Updated just now' : 'Alex-Rivera.pdf')}
          >
            {resumeNote.endsWith('.pdf') ? 'Edit resume' : 'Upload again'}
          </button>
        </section>

        <section>
          <p className="kicker">Links</p>
          <ul className="links">
            <li>
              <span>LinkedIn</span>
              <strong>linkedin.com/in/alex-rivera</strong>
            </li>
            <li>
              <span>Portfolio</span>
              <strong>alexrivera.work</strong>
            </li>
          </ul>
        </section>

        <section>
          <p className="kicker">Looking for</p>
          <div className="chips">
            <span>Barista</span>
            <span>Host</span>
            <span>Junior builder</span>
          </div>
          <p className="meta">$19.40/hr · Hybrid · Austin</p>
        </section>

        <section className="plan-block">
          <p className="kicker">Your plan</p>
          <p className="xp-role">{PLAN_LABEL[plan]}</p>
          <p className="body">Swipe is free. Your agent applies on VIP.</p>
          {plan === 'vip' ? (
            <p className="meta">VIP is on. No charge in this demo.</p>
          ) : (
            <button type="button" className="btn-acid" onClick={() => openCheckout('vip')}>
              Upgrade to VIP
            </button>
          )}
        </section>
      </div>
    </section>
  )
}
