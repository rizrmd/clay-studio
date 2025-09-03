import { useAuth } from "@/hooks/use-auth";
import { 
  StepProgressCard,
  ClientCreateForm,
  ClaudeSetupStep,
  SetupCompleteStep,
  CreateFirstUserStep
} from "@/components/setup";
import { useClaudeSetup } from "@/hooks/use-claude-setup";

export function SetupPage() {
  const { user, firstClient, needsInitialSetup } = useAuth();
  
  const {
    error,
    setError,
    claudeSetupResponse,
    setupProgress,
    cliOutput,
    updateClaudeSetupResponse
  } = useClaudeSetup({
    clientId: firstClient?.id,
    clientStatus: firstClient?.status
  });

  // Determine current step:
  // 0: Create client
  // 1: Claude setup
  // 2: Create first user
  // 3: Complete (but since isSetupComplete now requires projects, this won't be reached)
  const currentStep = !firstClient
    ? 0
    : firstClient.status !== "active"
    ? 1
    : !user
    ? 2
    : 3;
    
  const handleTokenSuccess = (message: string) => {
    updateClaudeSetupResponse({ message })
  }
  
  const handleTokenError = (error: string) => {
    setError(error)
  }

  const renderCurrentStep = () => {
    switch (currentStep) {
      case 0:
        return (
          <ClientCreateForm
            success={false}
            claudeSetupResponse={claudeSetupResponse}
          />
        )

      case 1:
        return (
          <ClaudeSetupStep
            claudeSetupResponse={claudeSetupResponse}
            firstClient={firstClient ? {
              id: firstClient.id,
              status: firstClient.status
            } : undefined}
            setupProgress={setupProgress}
            cliOutput={cliOutput}
            error={error}
            onTokenSuccess={handleTokenSuccess}
            onTokenError={handleTokenError}
          />
        )

      case 2:
        return (
          <CreateFirstUserStep 
            onSuccess={() => window.location.reload()}
          />
        )
        
      case 3:
        return <SetupCompleteStep />

      default:
        return null
    }
  }

  return (
    <div className="min-h-screen bg-background">
      <div className="container max-w-4xl mx-auto py-8 px-4">
        <div className="mb-8">
          <h1 className="text-3xl font-bold mb-2">
            {needsInitialSetup ? "Welcome to Clay Studio" : "Complete Setup"}
          </h1>
          <p className="text-muted-foreground">
            {needsInitialSetup
              ? "Create your first client to get started."
              : "Complete your Clay Studio setup."}
          </p>
        </div>

        <div className="grid gap-8 lg:grid-cols-3">
          <div className="lg:col-span-1">
            <StepProgressCard
              currentStep={currentStep}
              user={user || undefined}
              firstClient={firstClient || undefined}
            />
          </div>
          <div className="lg:col-span-2">
            {renderCurrentStep()}
          </div>
        </div>
      </div>
    </div>
  );
}