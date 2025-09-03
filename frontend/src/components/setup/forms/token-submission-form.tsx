import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Loader2 } from "lucide-react";
import axios from "@/lib/utils/axios";

interface TokenSubmissionFormProps {
  clientId?: string;
  onSuccess: (message: string) => void;
  onError: (error: string) => void;
}

export function TokenSubmissionForm({
  clientId,
  onSuccess,
  onError,
}: TokenSubmissionFormProps) {
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.target as HTMLFormElement);
    const token = formData.get("claude_token") as string;

    if (!token?.trim()) {
      onError("Please enter a valid token");
      return;
    }

    try {
      setIsLoading(true);
      onError("");
      const response = await axios.post("/claude/token", {
        client_id: clientId,
        claude_token: token.trim(),
      });

      if (response.data.success) {
        onSuccess(
          "Token submitted successfully! Proceeding to create admin account..."
        );
        setIsLoading(false);
        // Reload immediately to go to the next step (create admin account)
        setTimeout(() => window.location.reload(), 1000);
      }
    } catch (err: any) {
      onError(err.response?.data?.error || "Failed to submit token");
      setIsLoading(false);
    }
  };

  return (
    <div className="mt-4 p-6 bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-950/30 dark:to-emerald-950/30 rounded-lg border border-green-200 dark:border-green-800">
      <h4 className="text-lg font-semibold mb-2">
        🔑 Submit Authentication Token
      </h4>
      <p className="text-sm text-muted-foreground mb-4">
        After authenticating with Claude, you'll receive a token. Paste it
        below:
      </p>
      <form onSubmit={handleSubmit}>
        <div className="space-y-3">
          <Input
            name="claude_token"
            type="text"
            placeholder="Paste your Claude token here..."
            className="font-mono"
            required
          />
          <Button type="submit" disabled={isLoading} className="w-full">
            {isLoading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Submitting Token...
              </>
            ) : (
              "Submit Token"
            )}
          </Button>
        </div>
      </form>
    </div>
  );
}
