import { cn } from "@/lib/utils"

function Skeleton({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("animate-pulse rounded-md bg-primary/10 will-change-opacity", className)}
      {...props}
      style={{
        animationDuration: "2s",
        animationTimingFunction: "ease-in-out",
        ...props.style
      }}
    />
  )
}

export { Skeleton }
