import { useEffect } from 'react'
import { useSnapshot } from 'valtio'
import { ClientRootResponse, rootService } from '@/lib/services/root-service'
import { clientDetailDialogStore, clientDetailDialogActions } from '@/lib/store/ui/client-detail-dialog-store'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Switch } from '@/components/ui/switch'
import { 
  Globe, 
  Save, 
  X, 
  Plus,
  Key,
  Users,
  MessageSquare,
  Folder
} from 'lucide-react'
import { format } from 'date-fns'
import { Alert, AlertDescription } from '@/components/ui/alert'

interface ClientDetailDialogProps {
  client: ClientRootResponse
  open: boolean
  onOpenChange: (open: boolean) => void
  onUpdate: () => void
}

export function ClientDetailDialog({
  client,
  open,
  onOpenChange,
  onUpdate
}: ClientDetailDialogProps) {
  const dialogSnapshot = useSnapshot(clientDetailDialogStore)

  // Initialize state when client changes
  useEffect(() => {
    clientDetailDialogActions.initializeFromClient(client)
  }, [client])

  // Parse config for UI controls
  const configObj = typeof client.config === 'object' ? client.config : {}

  const handleSaveDetails = async () => {
    try {
      clientDetailDialogActions.setLoading(true)
      clientDetailDialogActions.setError(null)
      const updateData: any = { name: dialogSnapshot.name }
      if (dialogSnapshot.description && dialogSnapshot.description.trim()) {
        updateData.description = dialogSnapshot.description
      }
      await rootService.updateClient(client.id, updateData)
      onUpdate()
      onOpenChange(false)
    } catch (err: any) {
      const errorMsg = typeof err.response?.data?.error === 'string'
        ? err.response?.data?.error
        : (err.response?.data?.error?.brief || err.response?.data?.error?.name || 'Failed to update client')
      clientDetailDialogActions.setError(errorMsg)
    } finally {
      clientDetailDialogActions.setLoading(false)
    }
  }

  const handleSaveDomains = async () => {
    try {
      clientDetailDialogActions.setLoading(true)
      clientDetailDialogActions.setError(null)
      await rootService.updateClientDomains(client.id, [...dialogSnapshot.domains])
      onUpdate()
      onOpenChange(false)
    } catch (err: any) {
      const errorMsg = typeof err.response?.data?.error === 'string'
        ? err.response?.data?.error
        : (err.response?.data?.error?.brief || err.response?.data?.error?.name || 'Failed to update domains')
      clientDetailDialogActions.setError(errorMsg)
    } finally {
      clientDetailDialogActions.setLoading(false)
    }
  }

  const handleSaveConfig = async () => {
    try {
      clientDetailDialogActions.setLoading(true)
      clientDetailDialogActions.setError(null)
      const configObj = JSON.parse(dialogSnapshot.config)
      await rootService.updateClientConfig(client.id, configObj)
      onUpdate()
      onOpenChange(false)
    } catch (err: any) {
      if (err instanceof SyntaxError) {
        clientDetailDialogActions.setError('Invalid JSON format')
      } else {
        const errorMsg = typeof err.response?.data?.error === 'string'
          ? err.response?.data?.error
          : (err.response?.data?.error?.brief || err.response?.data?.error?.name || 'Failed to update configuration')
        clientDetailDialogActions.setError(errorMsg)
      }
    } finally {
      clientDetailDialogActions.setLoading(false)
    }
  }

  const handleSaveRegistrationSettings = async () => {
    try {
      clientDetailDialogActions.setLoading(true)
      clientDetailDialogActions.setError(null)
      const updatedConfig = {
        ...configObj,
        registration_enabled: dialogSnapshot.registrationEnabled,
        require_invite_code: dialogSnapshot.requireInviteCode,
        invite_code: dialogSnapshot.inviteCode || undefined
      }
      await rootService.updateClientConfig(client.id, updatedConfig)
      onUpdate()
      onOpenChange(false)
    } catch (err: any) {
      const errorMsg = typeof err.response?.data?.error === 'string'
        ? err.response?.data?.error
        : (err.response?.data?.error?.brief || err.response?.data?.error?.name || 'Failed to update registration settings')
      clientDetailDialogActions.setError(errorMsg)
    } finally {
      clientDetailDialogActions.setLoading(false)
    }
  }

  const handleAddDomain = () => {
    clientDetailDialogActions.addDomain(dialogSnapshot.newDomain)
  }

  const handleRemoveDomain = (domain: string) => {
    clientDetailDialogActions.removeDomain(domain)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Client Details: {client.name}</DialogTitle>
          <DialogDescription>
            Client ID: {client.id}
          </DialogDescription>
        </DialogHeader>

        {dialogSnapshot.error && (
          <Alert variant="destructive">
            <AlertDescription>
              {typeof dialogSnapshot.error === 'string' ? dialogSnapshot.error : JSON.stringify(dialogSnapshot.error)}
            </AlertDescription>
          </Alert>
        )}

        <Tabs value={dialogSnapshot.activeTab} onValueChange={clientDetailDialogActions.setActiveTab}>
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="details">Details</TabsTrigger>
            <TabsTrigger value="domains">Domains</TabsTrigger>
            <TabsTrigger value="config">Configuration</TabsTrigger>
            <TabsTrigger value="info">Info</TabsTrigger>
          </TabsList>

          <TabsContent value="details" className="space-y-4">
            <div className="space-y-4">
              <div>
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  value={dialogSnapshot.name}
                  onChange={(e) => clientDetailDialogActions.setName(e.target.value)}
                  placeholder="Client name"
                />
              </div>

              <div>
                <Label htmlFor="description">Description</Label>
                <Textarea
                  id="description"
                  value={dialogSnapshot.description}
                  onChange={(e) => clientDetailDialogActions.setDescription(e.target.value)}
                  placeholder="Client description"
                  rows={3}
                />
              </div>

              <div>
                <Label>Status</Label>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={client.status === 'active' ? 'default' : 'secondary'}>
                    {client.status}
                  </Badge>
                  {client.hasClaudeToken && (
                    <Badge variant="outline">
                      <Key className="h-3 w-3 mr-1" />
                      Claude Token
                    </Badge>
                  )}
                </div>
              </div>

              <div className="flex justify-end gap-2">
                <Button variant="outline" onClick={() => onOpenChange(false)}>
                  Cancel
                </Button>
                 <Button onClick={handleSaveDetails} disabled={dialogSnapshot.loading}>
                   <Save className="h-4 w-4 mr-2" />
                   Save Changes
                 </Button>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="domains" className="space-y-4">
            <div className="space-y-4">
              <div>
                <Label>Allowed Domains</Label>
                <p className="text-sm text-muted-foreground mt-1">
                  Configure which domains can access this client
                </p>
              </div>

              <div className="flex gap-2">
                 <Input
                   value={dialogSnapshot.newDomain}
                   onChange={(e) => clientDetailDialogActions.setNewDomain(e.target.value)}
                   placeholder="Enter domain (e.g., example.com)"
                   onKeyPress={(e) => e.key === 'Enter' && handleAddDomain()}
                 />
                 <Button onClick={handleAddDomain} size="sm">
                   <Plus className="h-4 w-4" />
                 </Button>
               </div>

               <div className="space-y-2">
                 {dialogSnapshot.domains.length === 0 ? (
                   <p className="text-sm text-muted-foreground">
                     No domains configured (accessible from any domain)
                   </p>
                 ) : (
                   dialogSnapshot.domains.map((domain: string) => (
                     <div key={domain} className="flex items-center justify-between p-2 border rounded">
                       <div className="flex items-center gap-2">
                         <Globe className="h-4 w-4 text-muted-foreground" />
                         <span className="text-sm">{domain}</span>
                       </div>
                       <Button
                         variant="ghost"
                         size="sm"
                         onClick={() => handleRemoveDomain(domain)}
                       >
                         <X className="h-4 w-4" />
                       </Button>
                     </div>
                   ))
                 )}
               </div>

               <div className="flex justify-end gap-2">
                 <Button variant="outline" onClick={() => onOpenChange(false)}>
                   Cancel
                 </Button>
                 <Button onClick={handleSaveDomains} disabled={dialogSnapshot.loading}>
                   <Save className="h-4 w-4 mr-2" />
                   Save Domains
                 </Button>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="config" className="space-y-4">
            <div className="space-y-6">
              {/* Registration Settings */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Registration Settings</CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label htmlFor="registration-enabled">Enable Registration</Label>
                      <p className="text-sm text-muted-foreground">
                        Allow new users to create accounts
                      </p>
                    </div>
                     <Switch
                       id="registration-enabled"
                       checked={dialogSnapshot.registrationEnabled}
                       onCheckedChange={clientDetailDialogActions.setRegistrationEnabled}
                     />
                   </div>

                   {dialogSnapshot.registrationEnabled && (
                     <>
                       <div className="flex items-center justify-between">
                         <div className="space-y-1">
                           <Label htmlFor="require-invite">Require Invite Code</Label>
                           <p className="text-sm text-muted-foreground">
                             Users must provide a code to register
                           </p>
                         </div>
                         <Switch
                           id="require-invite"
                           checked={dialogSnapshot.requireInviteCode}
                           onCheckedChange={clientDetailDialogActions.setRequireInviteCode}
                         />
                       </div>

                       {dialogSnapshot.requireInviteCode && (
                         <div>
                           <Label htmlFor="invite-code">Invite Code</Label>
                           <Input
                             id="invite-code"
                             type="text"
                             value={dialogSnapshot.inviteCode}
                             onChange={(e) => clientDetailDialogActions.setInviteCode(e.target.value)}
                             placeholder="Enter invite code"
                             className="mt-1"
                           />
                           <p className="text-xs text-muted-foreground mt-1">
                             Users will need this code to register
                           </p>
                         </div>
                       )}
                     </>
                   )}

                   <div className="flex justify-end gap-2 pt-2">
                     <Button onClick={handleSaveRegistrationSettings} disabled={dialogSnapshot.loading}>
                       <Save className="h-4 w-4 mr-2" />
                       Save Registration Settings
                     </Button>
                  </div>
                </CardContent>
              </Card>

              {/* Advanced Configuration */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Advanced Configuration</CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div>
                    <Label>Raw Configuration (JSON)</Label>
                    <p className="text-sm text-muted-foreground mt-1">
                      Edit the raw configuration for advanced settings
                    </p>
                  </div>

                   <Textarea
                     value={dialogSnapshot.config}
                     onChange={(e) => clientDetailDialogActions.setConfig(e.target.value)}
                     className="font-mono text-xs"
                     rows={10}
                   />

                   <div className="flex justify-end gap-2">
                     <Button variant="outline" onClick={() => onOpenChange(false)}>
                       Cancel
                     </Button>
                     <Button onClick={handleSaveConfig} disabled={dialogSnapshot.loading}>
                       <Save className="h-4 w-4 mr-2" />
                       Save Raw Configuration
                     </Button>
                  </div>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="info" className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">Statistics</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Users className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm">Users</span>
                    </div>
                    <span className="font-medium">{client.userCount}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Folder className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm">Projects</span>
                    </div>
                    <span className="font-medium">{client.projectCount}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <MessageSquare className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm">Conversations</span>
                    </div>
                    <span className="font-medium">{client.conversationCount}</span>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">Timestamps</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div>
                    <p className="text-xs text-muted-foreground">Created</p>
                    <p className="text-sm">
                      {format(new Date(client.createdAt), 'PPp')}
                    </p>
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground">Updated</p>
                    <p className="text-sm">
                      {format(new Date(client.updatedAt), 'PPp')}
                    </p>
                  </div>
                  {client.deletedAt && (
                    <div>
                      <p className="text-xs text-muted-foreground">Deleted</p>
                      <p className="text-sm">
                        {format(new Date(client.deletedAt), 'PPp')}
                      </p>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>

            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Installation</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  <div>
                    <p className="text-xs text-muted-foreground">Install Path</p>
                    <p className="text-sm font-mono">{client.installPath}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}