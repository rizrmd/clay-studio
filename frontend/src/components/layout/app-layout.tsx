import { AppHeader } from './app-header'

interface AppLayoutProps {
  children: React.ReactNode
}

export function AppLayout({ children }: AppLayoutProps) {
  return (
    <div className="min-h-screen bg-background">
      <AppHeader />
      <main>{children}</main>
    </div>
  )
}