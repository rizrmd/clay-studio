# Chat Components Structure

This directory contains all chat-related components organized into logical groups.

## Directory Structure

```
chat/
├── main/           # Main chat container component
│   └── chat.tsx    # Primary chat component that orchestrates the UI
├── sidebar/        # Sidebar components
│   ├── conversation-sidebar.tsx  # List of conversations
│   ├── file-sidebar.tsx         # File browser sidebar
│   └── file-manager.tsx         # File management utilities
├── input/          # Input components
│   └── multimodal-input.tsx     # Chat input with file attachments
├── display/        # Message display components
│   ├── messages.tsx              # Message list display
│   ├── suggestion-cards.tsx     # Suggestion cards UI
│   ├── welcome-area.tsx          # Welcome screen
│   └── tools-display.tsx        # Tools visualization
└── index.ts        # Main export file
```

## Component Groups

### Main (`/main`)
The primary chat component that brings together all other components.

### Sidebar (`/sidebar`)
Components related to navigation and file management:
- **ConversationSidebar**: Displays list of conversations with rename/delete functionality
- **FileSidebar**: Browse and manage project files
- **FileManager**: Utilities for file operations

### Input (`/input`)
User input components:
- **MultimodalInput**: Rich input component supporting text and file attachments

### Display (`/display`)
Components for displaying content:
- **Messages**: Renders chat messages with markdown support
- **SuggestionCards**: Shows contextual suggestions
- **WelcomeArea**: Initial screen when no conversation is active
- **ToolsDisplay**: Visualizes available tools and their usage

## Usage

Import components from the main index:

```typescript
import { Chat, ConversationSidebar, Messages } from '@/components/chat'
```

Or import from specific groups:

```typescript
import { MultimodalInput } from '@/components/chat/input'
import { Messages, WelcomeArea } from '@/components/chat/display'
```