import { SuggestionCards } from './suggestion-cards'

export function WelcomeArea() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center p-8">
      <div className="text-center mb-12">
        <h1 className="text-3xl font-bold text-foreground mb-2">
          Hello there!
        </h1>
        <p className="text-lg text-muted-foreground">
          How can I help you today?
        </p>
      </div>
      
      <SuggestionCards />
    </div>
  )
}