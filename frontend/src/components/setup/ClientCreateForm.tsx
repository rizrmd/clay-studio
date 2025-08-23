import { useState } from "react";
import { useForm } from "react-hook-form";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { CheckCircle2, Loader2 } from "lucide-react";
import axios from "@/lib/axios";

interface ClientCreateData {
  name: string;
}

interface ClientCreateFormProps {
  success: boolean;
  claudeSetupResponse: any;
}

export function ClientCreateForm({
  success,
  claudeSetupResponse,
}: ClientCreateFormProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState("");

  const form = useForm<ClientCreateData>({
    defaultValues: {
      name: "",
    },
  });

  const onSubmit = async (data: ClientCreateData) => {
    setIsLoading(true);
    setError("");

    try {
      const response = await axios.post("/api/clients", {
        name: data.name,
        description: `Client for ${data.name}`,
      });

      if (response.data) {
        setTimeout(() => {
          window.location.reload();
        }, 1000);
      }
    } catch (err: any) {
      setError(err.response?.data?.error || "Failed to create client");
      setIsLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Create Your Client</CardTitle>
        <CardDescription>
          Choose a name for your organization or workspace.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {success && claudeSetupResponse ? (
          <div className="text-center py-8">
            <CheckCircle2 className="h-12 w-12 text-green-600 mx-auto mb-4" />
            <h3 className="text-lg font-medium mb-2">
              Client Created & Claude Code Ready!
            </h3>
            <p className="text-muted-foreground mb-4">
              {claudeSetupResponse.message}
            </p>
            <Button onClick={() => window.location.reload()}>
              Continue to App
            </Button>
          </div>
        ) : (
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
              {error && (
                <Alert variant="destructive">
                  <AlertDescription>{error}</AlertDescription>
                </Alert>
              )}

              <FormField
                control={form.control}
                name="name"
                rules={{
                  required: "Client name is required",
                  minLength: {
                    value: 2,
                    message: "Client name must be at least 2 characters",
                  },
                }}
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Client Name</FormLabel>
                    <FormControl>
                      <Input
                        placeholder="Enter your organization name"
                        {...field}
                        disabled={isLoading}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <Button type="submit" disabled={isLoading} className="w-full">
                {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Create Client
              </Button>
            </form>
          </Form>
        )}
      </CardContent>
    </Card>
  );
}
