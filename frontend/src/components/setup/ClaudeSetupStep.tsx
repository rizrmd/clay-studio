import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { CheckCircle2, Settings } from "lucide-react";
import { TerminalOutput } from "./TerminalOutput";
import { AuthUrlDisplay } from "./AuthUrlDisplay";
import { TokenSubmissionForm } from "./TokenSubmissionForm";

interface ClaudeSetupStepProps {
  claudeSetupResponse: any;
  firstClient?: {
    id: string;
    status?: string;
  };
  setupProgress: string;
  cliOutput: string[];
  error: string;
  onTokenSuccess: (message: string) => void;
  onTokenError: (error: string) => void;
}

export function ClaudeSetupStep({
  claudeSetupResponse,
  firstClient,
  setupProgress,
  cliOutput,
  error,
  onTokenSuccess,
  onTokenError,
}: ClaudeSetupStepProps) {
  const isComplete = claudeSetupResponse?.message?.includes("complete");

  // Only use auth URL from EventSource (claudeSetupResponse), not from CLI output
  // This ensures we only show auth UI when properly detected by backend
  const authUrl = claudeSetupResponse?.auth_url;

  // Hide terminal once auth URL is detected from EventSource
  const showTerminal =
    !authUrl &&
    (firstClient?.status === "installing" ||
      firstClient?.status === "pending" ||
      setupProgress);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Claude Code Setup</CardTitle>
        <CardDescription>
          Setting up Claude Code environment for your client.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isComplete ? (
          <div className="text-center py-8">
            <CheckCircle2 className="h-12 w-12 text-green-600 mx-auto mb-4" />
            <h3 className="text-lg font-medium mb-2">
              Claude Code Setup Complete!
            </h3>
            <p className="text-muted-foreground mb-4">
              {claudeSetupResponse.message || "Token submitted successfully! Proceeding to create admin account..."}
            </p>
            <p className="text-sm text-muted-foreground">
              Redirecting...
            </p>
          </div>
        ) : (
          <>
            {showTerminal && (
              <TerminalOutput
                cliOutput={cliOutput}
                setupProgress={setupProgress}
              />
            )}

            {authUrl && (
              <>
                <AuthUrlDisplay authUrl={authUrl} />
                <TokenSubmissionForm
                  clientId={firstClient?.id}
                  onSuccess={onTokenSuccess}
                  onError={onTokenError}
                />
              </>
            )}

            {!showTerminal && !authUrl && firstClient?.status === "error" ? (
              <>
                <Settings className="h-12 w-12 text-red-600 mx-auto mb-4" />
                <h3 className="text-lg font-medium mb-2">Setup Error</h3>
                <p className="text-muted-foreground mb-4">
                  There was an error during Claude Code setup. Please try again.
                </p>
                {error && (
                  <Alert variant="destructive" className="mb-4">
                    <AlertDescription>{error}</AlertDescription>
                  </Alert>
                )}
                <Button onClick={() => window.location.reload()}>
                  Retry Setup
                </Button>
              </>
            ) : (
              <></>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
