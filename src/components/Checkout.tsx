import { useEffect } from 'react'
import { go } from '../routes.ts'
import { useBagz } from '../store.tsx'
import { IconBack } from './Icons.tsx'

export function Checkout() {
  const { offer, receipt, confirmCheckout } = useBagz()
  const name = offer === 'vip' ? 'VIP' : 'Pro'
  const price = offer === 'vip' ? '$8.99' : '$2.99'

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') go('profile')
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  return (
    <section className="screen checkout" data-screen="checkout">
      <button type="button" className="back" onClick={() => go('profile')}>
        <IconBack />
        Back
      </button>
      {receipt ? (
        <div className="check">
          <p className="kicker">Not charged</p>
          <h1 className="display display-m">
            You’re on
            <br />
            <span>{name}.</span>
          </h1>
          <p className="body">No charge. No card. No forms.</p>
          <dl className="receipt">
            <div>
              <dt>Plan</dt>
              <dd>{name}</dd>
            </div>
            <div>
              <dt>Price</dt>
              <dd>{price}/month</dd>
            </div>
            <div>
              <dt>Payment</dt>
              <dd>Skipped</dd>
            </div>
          </dl>
          <button type="button" className="btn-acid" onClick={() => go(offer === 'vip' ? 'chat' : 'profile')}>
            {offer === 'vip' ? 'Watch your agent →' : 'Back to profile'}
          </button>
        </div>
      ) : (
        <div className="check">
          <p className="kicker">Demo checkout</p>
          <h1 className="display display-m">
            Confirm
            <br />
            <span>{name}.</span>
          </h1>
          <p className="pay price-hero">{price}</p>
          <p className="body">No charge. Nothing is stored or sent.</p>
          <button type="button" className="btn-acid" onClick={confirmCheckout}>
            Confirm {name}
          </button>
          <button type="button" className="btn-line" onClick={() => go('profile')}>
            Cancel
          </button>
        </div>
      )}
    </section>
  )
}
