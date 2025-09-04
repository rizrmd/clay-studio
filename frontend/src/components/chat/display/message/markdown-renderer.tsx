import { memo } from "react";
import ReactMarkdown from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import rehypeRaw from "rehype-raw";
import remarkGfm from "remark-gfm";
import "github-markdown-css/github-markdown-light.css";

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

export const MarkdownRenderer = memo(({ content, className }: MarkdownRendererProps) => {
  return (
    <div className={className}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight, rehypeRaw]}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});

MarkdownRenderer.displayName = "MarkdownRenderer";