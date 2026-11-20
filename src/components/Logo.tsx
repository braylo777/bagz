export function Logo({ height = 48 }: { height?: number }) {
  return (
    <img
      className="logo"
      src={`${import.meta.env.BASE_URL}brand/bagz-logo-dark.svg`}
      alt="BAGZ"
      style={{ height, width: 'auto' }}
      draggable={false}
    />
  )
}
