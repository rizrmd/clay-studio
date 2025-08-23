import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { CheckCircle2, Settings, Rocket, User, LucideIcon } from 'lucide-react'

interface Step {
  title: string
  description: string
  icon: LucideIcon
}

interface StepProgressCardProps {
  currentStep: number
  user?: {
    username: string
  }
  firstClient?: {
    name: string
  }
}

export function StepProgressCard({ currentStep, user, firstClient }: StepProgressCardProps) {
  const steps: Step[] = [
    {
      title: 'Create Client',
      description: 'Name your organization',
      icon: Rocket,
    },
    {
      title: 'Initialize Claude Code',
      description: 'Complete Claude Code integration',
      icon: Settings,
    },
    {
      title: 'Create Admin Account',
      description: 'Set up your administrator account',
      icon: User,
    },
    {
      title: 'Ready to Go',
      description: 'Start using Clay Studio',
      icon: CheckCircle2,
    },
  ]

  return (
    <div>
      <Card>
        <CardHeader>
          <CardTitle>Setup Progress</CardTitle>
          <CardDescription>
            Complete these steps to get started
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {steps.map((step, index) => {
              const Icon = step.icon
              const isActive = index === currentStep
              const isComplete = index < currentStep
              
              return (
                <div
                  key={index}
                  className={`flex items-start gap-3 ${
                    isActive ? 'text-primary' : isComplete ? 'text-muted-foreground' : 'text-muted-foreground/50'
                  }`}
                >
                  <div className={`mt-0.5 ${isActive ? 'text-primary' : ''}`}>
                    {isComplete ? (
                      <CheckCircle2 className="h-5 w-5 text-green-600" />
                    ) : (
                      <Icon className="h-5 w-5" />
                    )}
                  </div>
                  <div>
                    <p className="font-medium">{step.title}</p>
                    <p className="text-sm text-muted-foreground">
                      {step.description}
                    </p>
                  </div>
                </div>
              )
            })}
          </div>
        </CardContent>
      </Card>

      {user && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle>Account</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 text-sm">
              <div>
                <span className="text-muted-foreground">Username: </span>
                <span className="font-medium">{user.username}</span>
              </div>
              {firstClient && (
                <div>
                  <span className="text-muted-foreground">Client: </span>
                  <span className="font-medium">{firstClient.name}</span>
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}