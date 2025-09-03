import { Bot } from "lucide-react";

export function WelcomeScreen() {
  return (
    <div className="flex flex-1 w-full h-full flex-col items-center justify-start md:justify-center text-center pt-12 md:pt-0">
      <div className="flex h-20 w-20 items-center justify-center rounded-full bg-muted">
        <Bot className="h-10 w-10" />
      </div>
      <h2 className="mt-4 text-xl font-semibold">Welcome to Clay Studio</h2>
      <p className="mt-2 text-muted-foreground px-4">
        I'm here to help you analyze your data. What would you like to explore?
      </p>
    </div>
  );
}