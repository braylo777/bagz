import { Logo } from './Logo.tsx'
import { go } from '../routes.ts'

export function Splash() {
  return (
    <section className="screen splash" data-screen="splash">
      <div className="glow glow-violet" aria-hidden="true" />
      <div className="glow glow-green" aria-hidden="true" />
      <div className="splash-copy">
        <Logo height={56} />
        <h1 className="display">
          Swipe to
          <br />
          <span>get hired.</span>
        </h1>
      </div>
      <button type="button" className="btn-acid" onClick={() => go('bagz')}>
        Get started →
      </button>
    </section>
  )
}
