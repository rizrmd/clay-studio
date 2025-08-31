// import { Info } from "lucide-react";
// import { cn } from "@/lib/utils";
// import {
//   Tooltip,
//   TooltipContent,
//   TooltipProvider,
//   TooltipTrigger,
// } from "@/components/ui/tooltip";
// import { Progress } from "@/components/ui/progress";

interface ContextIndicatorProps {
  contextUsage?: {
    totalChars: number;
    maxChars: number;
    percentage: number;
    messageCount: number;
    needsCompaction: boolean;
  };
  className?: string;
}

export function ContextIndicator({}: ContextIndicatorProps) {
  return null;
  // if (!contextUsage) return null;

  // const { percentage, messageCount, needsCompaction, totalChars, maxChars } = contextUsage;

  // // Determine color based on usage
  // const getColor = () => {
  //   if (percentage < 50) return 'text-green-600 dark:text-green-400';
  //   if (percentage < 75) return 'text-yellow-600 dark:text-yellow-400';
  //   return 'text-orange-600 dark:text-orange-400';
  // };

  // const getProgressColor = () => {
  //   if (percentage < 50) return 'bg-green-600 dark:bg-green-400';
  //   if (percentage < 75) return 'bg-yellow-600 dark:bg-yellow-400';
  //   return 'bg-orange-600 dark:bg-orange-400';
  // };

  // const formatNumber = (num: number) => {
  //   if (num < 1000) return num.toString();
  //   if (num < 1000000) return `${(num / 1000).toFixed(1)}k`;
  //   return `${(num / 1000000).toFixed(1)}M`;
  // };

  // return (
  //   <TooltipProvider>
  //     <Tooltip>
  //       <TooltipTrigger asChild>
  //         <div className={cn('flex items-center gap-2 text-xs', className)}>
  //           <div className="flex items-center gap-1">
  //             <Info className={cn('h-3 w-3', getColor())} />
  //             <span className={cn('font-medium', getColor())}>
  //               {Math.round(percentage)}%
  //             </span>
  //             <span className="text-muted-foreground">
  //               ({messageCount} {messageCount === 1 ? 'msg' : 'msgs'})
  //             </span>
  //           </div>
  //           {needsCompaction && (
  //             <span className="text-xs text-muted-foreground italic">
  //               compacting
  //             </span>
  //           )}
  //         </div>
  //       </TooltipTrigger>
  //       <TooltipContent className="max-w-xs">
  //         <div className="space-y-2 p-1">
  //           <div className="font-medium">Context Usage</div>
  //           <Progress
  //             value={percentage}
  //             className="h-2 w-full"
  //             indicatorClassName={getProgressColor()}
  //           />
  //           <div className="text-xs space-y-1">
  //             <div>
  //               <span className="text-muted-foreground">Characters:</span>{' '}
  //               {formatNumber(totalChars)} / {formatNumber(maxChars)}
  //             </div>
  //             <div>
  //               <span className="text-muted-foreground">Messages:</span>{' '}
  //               {messageCount}
  //             </div>
  //             {needsCompaction && (
  //               <div className="text-yellow-600 dark:text-yellow-400">
  //                 Claude will automatically compact older messages to stay within limits
  //               </div>
  //             )}
  //             {percentage > 90 && (
  //               <div className="text-orange-600 dark:text-orange-400">
  //                 Context is nearly full. Consider starting a new conversation for optimal performance.
  //               </div>
  //             )}
  //           </div>
  //         </div>
  //       </TooltipContent>
  //     </Tooltip>
  //   </TooltipProvider>
  // );
}
