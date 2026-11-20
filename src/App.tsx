import { Chat } from './components/Chat.tsx'
import { Checkout } from './components/Checkout.tsx'
import { Deck } from './components/Deck.tsx'
import { Profile } from './components/Profile.tsx'
import { Shell } from './components/Shell.tsx'
import { Splash } from './components/Splash.tsx'
import { useRoute } from './routes.ts'
import { StoreProvider } from './store.tsx'

export default function App() {
  return (
    <StoreProvider>
      <Root />
    </StoreProvider>
  )
}

function Root() {
  const route = useRoute()
  return (
    <Shell route={route}>
      {route === 'splash' ? <Splash /> : null}
      {route === 'bagz' ? <Deck /> : null}
      {route === 'chat' ? <Chat /> : null}
      {route === 'profile' ? <Profile /> : null}
      {route === 'checkout' ? <Checkout /> : null}
    </Shell>
  )
}
