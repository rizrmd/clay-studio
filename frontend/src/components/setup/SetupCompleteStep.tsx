import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { CheckCircle2 } from 'lucide-react'

export function SetupCompleteStep() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Setup Complete!</CardTitle>
        <CardDescription>
          Your Clay Studio client is ready to use.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="text-center py-8">
          <CheckCircle2 className="h-12 w-12 text-green-600 mx-auto mb-4" />
          <h3 className="text-lg font-medium mb-2">All Set!</h3>
          <p className="text-muted-foreground mb-4">
            Your client is active and ready to use.
          </p>
          <Button onClick={() => window.location.reload()}>
            Continue to App
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}