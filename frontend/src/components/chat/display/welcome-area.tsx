import { SuggestionCards } from './suggestion-cards'

export function WelcomeArea() {
  return (
    <div className="flex flex-1 flex-col items-center justify-start md:justify-center p-4 md:p-8 pt-12 md:pt-8">
      <div className="text-center mb-8 md:mb-12">
        <h1 className="text-2xl md:text-3xl font-bold text-foreground mb-2">
          Hello there!
        </h1>
        <p className="text-base md:text-lg text-muted-foreground">
          How can I help you today?
        </p>
      </div>
      
      <SuggestionCards />
    </div>
  )
}