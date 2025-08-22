import { Card } from '@/components/ui/card'

interface SuggestionCard {
  title: string
  subtitle: string
}

const suggestions: SuggestionCard[] = [
  {
    title: "What are the advantages",
    subtitle: "of using Next.js?"
  },
  {
    title: "Write code to",
    subtitle: "demonstrate dijkstra's algorithm"
  },
  {
    title: "Help me write an essay",
    subtitle: "about silicon valley"
  },
  {
    title: "What is the weather",
    subtitle: "in San Francisco?"
  }
]

export function SuggestionCards() {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-w-2xl mx-auto mb-8">
      {suggestions.map((suggestion, index) => (
        <Card 
          key={index}
          className="p-4 cursor-pointer hover:bg-accent transition-colors border border-border"
        >
          <div className="text-sm font-medium text-foreground mb-1">
            {suggestion.title}
          </div>
          <div className="text-sm text-muted-foreground">
            {suggestion.subtitle}
          </div>
        </Card>
      ))}
    </div>
  )
}