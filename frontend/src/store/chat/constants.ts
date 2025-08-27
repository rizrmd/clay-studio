export const CONVERSATION_STATES = {
  IDLE: 'idle',
  LOADING: 'loading',
  STREAMING: 'streaming',
  ERROR: 'error',
  PROCESSING_QUEUE: 'processing_queue',
} as const;

export const MAX_RETRY_ATTEMPTS = 3;
export const RETRY_DELAY_MS = 1000;
export const CONVERSATION_CACHE_SIZE = 10; // Max conversations to keep in memory
export const MESSAGE_BATCH_SIZE = 50; // Messages to load at once
export const STREAM_TIMEOUT_MS = 60000; // 1 minute timeout for streaming