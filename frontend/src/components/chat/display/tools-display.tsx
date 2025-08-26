import { Wrench, Database, BarChart3, Brain, FileSearch, Upload } from 'lucide-react'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { Badge } from "@/components/ui/badge"
import type { ToolContext } from '@/types/chat'

interface ToolsDisplayProps {
  tools: ToolContext[]
  className?: string
}

const categoryIcons: Record<string, React.ElementType> = {
  sql: Database,
  time_series: BarChart3,
  statistics: BarChart3,
  machine_learning: Brain,
  data_quality: FileSearch,
  data_exploration: FileSearch,
  data_import: Upload,
  visualization: BarChart3,
  nlp: Brain,
  export: Upload,
}

const categoryColors: Record<string, string> = {
  sql: 'bg-blue-100 text-blue-800 border-blue-200',
  time_series: 'bg-purple-100 text-purple-800 border-purple-200',
  statistics: 'bg-indigo-100 text-indigo-800 border-indigo-200',
  machine_learning: 'bg-pink-100 text-pink-800 border-pink-200',
  data_quality: 'bg-yellow-100 text-yellow-800 border-yellow-200',
  data_exploration: 'bg-green-100 text-green-800 border-green-200',
  data_import: 'bg-orange-100 text-orange-800 border-orange-200',
  visualization: 'bg-cyan-100 text-cyan-800 border-cyan-200',
  nlp: 'bg-red-100 text-red-800 border-red-200',
  export: 'bg-gray-100 text-gray-800 border-gray-200',
}

export function ToolsDisplay({ tools, className = '' }: ToolsDisplayProps) {
  // Group tools by category
  const toolsByCategory = tools.reduce((acc, tool) => {
    if (!acc[tool.category]) {
      acc[tool.category] = []
    }
    acc[tool.category].push(tool)
    return acc
  }, {} as Record<string, ToolContext[]>)

  const totalTools = tools.length

  if (totalTools === 0) {
    return null
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`flex items-center gap-2 ${className}`}>
            <Wrench className="h-3.5 w-3.5 text-green-600" />
            <span className="text-sm font-medium text-green-600">
              {totalTools} {totalTools === 1 ? 'Tool' : 'Tools'} Available
            </span>
          </div>
        </TooltipTrigger>
        <TooltipContent className="max-w-md p-0" side="bottom" align="start">
          <div className="p-3 space-y-3">
            <div className="text-sm font-semibold">Available Analysis Tools</div>
            
            {/* Category badges */}
            <div className="flex flex-wrap gap-1.5">
              {Object.entries(toolsByCategory).map(([category, categoryTools]) => {
                const Icon = categoryIcons[category] || Wrench
                const colorClass = categoryColors[category] || 'bg-gray-100 text-gray-800 border-gray-200'
                
                return (
                  <Badge
                    key={category}
                    variant="outline"
                    className={`text-xs px-2 py-0.5 ${colorClass} flex items-center gap-1`}
                  >
                    <Icon className="h-3 w-3" />
                    <span>{category.replace(/_/g, ' ')}</span>
                    <span className="font-semibold">({categoryTools.length})</span>
                  </Badge>
                )
              })}
            </div>

            {/* Tool list by category */}
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {Object.entries(toolsByCategory).map(([category, categoryTools]) => (
                <div key={category} className="space-y-1">
                  <div className="text-xs font-medium text-muted-foreground capitalize">
                    {category.replace(/_/g, ' ')}
                  </div>
                  <div className="pl-2 space-y-0.5">
                    {categoryTools.slice(0, 3).map((tool, idx) => (
                      <div key={idx} className="text-xs text-foreground">
                        • {tool.name}
                      </div>
                    ))}
                    {categoryTools.length > 3 && (
                      <div className="text-xs text-muted-foreground">
                        ...and {categoryTools.length - 3} more
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>

            <div className="text-xs text-muted-foreground pt-2 border-t">
              Tools are automatically selected based on your data sources and project configuration
            </div>
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}