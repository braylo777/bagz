import type { ReactNode } from 'react'

type IconProps = { size?: number }

function Svg({ size = 22, children }: IconProps & { children: ReactNode }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      {children}
    </svg>
  )
}

const stroke = {
  stroke: 'currentColor',
  strokeWidth: 2,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
}

export function IconPass({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M7 7l10 10M17 7 7 17" {...stroke} />
    </Svg>
  )
}

export function IconApply({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M5 12h12M13 6l6 6-6 6" {...stroke} />
    </Svg>
  )
}

export function IconDeck({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <rect x="7" y="3.5" width="12" height="16" rx="3" {...stroke} />
      <path d="M6 8H5.2A2.2 2.2 0 0 0 3 10.2v8.1A2.2 2.2 0 0 0 5.2 20.5H14" {...stroke} />
    </Svg>
  )
}

export function IconAgent({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M5 8h9M5 12h14M5 16h7" {...stroke} />
    </Svg>
  )
}

export function IconChat({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M6 16.5 4 20l4.2-1.6A8.5 8.5 0 1 0 6 16.5z" {...stroke} />
    </Svg>
  )
}

export function IconMe({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <circle cx="12" cy="9" r="3.2" {...stroke} />
      <path d="M6.2 19.2c.9-2.6 2.8-3.9 5.8-3.9s4.9 1.3 5.8 3.9" {...stroke} />
    </Svg>
  )
}

export function IconTiers({ size = 22 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M12 4 19 8 12 12 5 8 12 4z" {...stroke} />
      <path d="M5 12.2 12 16.2 19 12.2" {...stroke} />
      <path d="M5 16.2 12 20.2 19 16.2" {...stroke} />
    </Svg>
  )
}

export function IconCheck({ size = 16 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M5 12.5 9.2 16.5 19 7.5" {...stroke} />
    </Svg>
  )
}

export function IconBack({ size = 20 }: IconProps = {}) {
  return (
    <Svg size={size}>
      <path d="M14.5 5.5 8 12l6.5 6.5" {...stroke} />
    </Svg>
  )
}
