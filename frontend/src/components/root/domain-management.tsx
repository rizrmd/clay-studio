import { rootService } from '@/lib/services/root-service'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Globe } from 'lucide-react'
import { DomainEditor } from '@/components/domains/domain-editor'

interface DomainManagementProps {
  clientId: string
  initialDomains: string[]
  onUpdate: () => void
  showCard?: boolean
  title?: string
}

export function DomainManagement({ 
  clientId, 
  initialDomains = [], 
  onUpdate,
  showCard = true,
  title = "Allowed Domains"
}: DomainManagementProps) {
  
  const handleSave = async (domains: string[]) => {
    await rootService.updateClientDomains(clientId, domains)
    onUpdate()
  }

  const content = (
    <DomainEditor
      initialDomains={initialDomains}
      onSave={handleSave}
      showSaveButtons={true}
    />
  )

  if (!showCard) {
    return content
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base flex items-center gap-2">
          <Globe className="h-4 w-4" />
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {content}
      </CardContent>
    </Card>
  )
}