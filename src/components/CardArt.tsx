export function Monogram({ letter }: { letter: string }) {
  return (
    <svg className="mono" viewBox="0 0 80 80" aria-hidden="true">
      <rect width="80" height="80" rx="22" fill="#1E1F24" stroke="#8B5CFF" strokeWidth="3" />
      <text x="40" y="54" textAnchor="middle" fontFamily="Anton, sans-serif" fontSize="40" fill="#FFFFFF">
        {letter}
      </text>
    </svg>
  )
}

export function Mascot({ id }: { id: string }) {
  const common = { fill: 'none', stroke: '#C6FF3D', strokeWidth: 2.4, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const }
  return (
    <svg className="mascot" viewBox="0 0 64 64" aria-hidden="true">
      {id === 'poster' ? <path d="M14 50 28 14l6 6L20 56zM34 20l16 16" {...common} /> : null}
      {id === 'carton' ? (
        <>
          <rect x="16" y="20" width="32" height="24" rx="8" {...common} />
          <circle cx="26" cy="32" r="3" {...common} />
          <circle cx="38" cy="32" r="3" {...common} />
          <path d="M24 40h16M24 16v4M40 16v4" {...common} />
        </>
      ) : null}
      {id === 'lace' || id === 'rack' ? <path d="M32 10 46 22 32 54 18 22z" {...common} /> : null}
      {id === 'ping' || id === 'quad' ? <path d="M12 40c8-16 32-16 40 0M32 16v28" {...common} /> : null}
      {id === 'spoke' ? <path d="M12 40h40M20 40a12 12 0 1 1 24 0" {...common} /> : null}
      {id === 'clip' ? <path d="M18 18h20l8 8v20H18zM38 18v8h8" {...common} /> : null}
      {id === 'salsa' || id === 'froth' ? <path d="M18 26h28l-4 24H22zM24 26c0-8 16-8 16 0" {...common} /> : null}
      {id === 'latebird' ? <path d="M16 22h32v20H16zM16 30h32" {...common} /> : null}
      {id === 'dayglow' ? <path d="M32 14v8M20 40h24M24 32h16v14H24z" {...common} /> : null}
    </svg>
  )
}

export function rarityOf(matchPct: number) {
  if (matchPct >= 90) return 'legendary'
  if (matchPct >= 85) return 'rare'
  return 'common'
}

export function stickerFor(job: { posted: string; mode: string; matchPct: number; payScore: number }) {
  if (job.matchPct >= 90) return 'HOT'
  if (job.mode === 'Remote') return 'REMOTE'
  if (job.posted.endsWith('h ago')) return 'NEW'
  if (job.payScore >= 80) return '$$$'
  return ''
}

export function scoreOf(job: { payScore: number; matchPct: number; growth: number; vibe: number }) {
  return Math.round((job.payScore + job.matchPct + job.growth + job.vibe) / 4)
}
