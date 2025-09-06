export interface Attachment {
  name?: string;
  contentType?: string;
  url: string;
}

export interface PartialToolCall {
  state: "partial-call";
  toolName: string;
}

export interface ToolCall {
  state: "call";
  toolName: string;
}

export interface ToolResult {
  state: "result";
  toolName: string;
  result: {
    __cancelled?: boolean;
    [key: string]: any;
  };
}

export type ToolInvocation = (PartialToolCall | ToolCall | ToolResult) & {
  id: string;
};

export interface ReasoningPart {
  type: "reasoning";
  reasoning: string;
}

export interface ToolInvocationPart {
  type: "tool-invocation";
  toolInvocation: ToolInvocation;
}

export interface TextPart {
  type: "text";
  text: string;
}

export interface SourcePart {
  type: "source";
  source?: any;
}

export interface FilePart {
  type: "file";
  mimeType: string;
  data: string;
}

export interface StepStartPart {
  type: "step-start";
}

export type MessagePart =
  | TextPart
  | ReasoningPart
  | ToolInvocationPart
  | SourcePart
  | FilePart
  | StepStartPart;

export interface Message {
  id: string;
  role: "user" | "assistant" | (string & {});
  content: string;
  createdAt?: Date;
  experimental_attachments?: Attachment[];
  toolInvocations?: ToolInvocation[];
  parts?: MessagePart[];
}