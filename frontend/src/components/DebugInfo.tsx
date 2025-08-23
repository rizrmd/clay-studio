import { useAuth } from '@/contexts/AuthContext'

export function DebugInfo() {
  const { user, firstClient, loading, isAuthenticated, isSetupComplete, needsInitialSetup } = useAuth()
  
  return (
    <div style={{ 
      position: 'fixed', 
      bottom: 10, 
      right: 10, 
      background: 'rgba(0,0,0,0.8)', 
      color: 'white', 
      padding: '10px',
      borderRadius: '5px',
      fontSize: '12px',
      fontFamily: 'monospace',
      maxWidth: '400px'
    }}>
      <div>Loading: {loading ? 'true' : 'false'}</div>
      <div>Authenticated: {isAuthenticated ? 'true' : 'false'}</div>
      <div>Setup Complete: {isSetupComplete ? 'true' : 'false'}</div>
      <div>Needs Initial Setup: {needsInitialSetup ? 'true' : 'false'}</div>
      <div>User: {user ? user.username : 'null'}</div>
      <div>Client ID: {firstClient?.id || 'null'}</div>
      <div>Client Name: {firstClient?.name || 'null'}</div>
      <div>Client Status: {firstClient?.status || 'null'}</div>
      <div>Should Show Setup: {firstClient && firstClient.status !== 'active' ? 'YES' : 'NO'}</div>
    </div>
  )
}