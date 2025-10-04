# Multi-User Projects API Guide

This document explains how to use the multi-user project features and conversation visibility controls.

## Overview

- **Projects** can now have multiple members (owners and regular members)
- **Conversations** can be private (creator only) or public (all project members)
- **Default behavior**: New conversations are private

## Project Members API

### List Project Members

```typescript
import { listProjectMembers } from "@/lib/api/project-members";

const members = await listProjectMembers(projectId);
// Returns: ProjectMember[]
// [
//   {
//     id: "uuid",
//     project_id: "project-123",
//     user_id: "user-456",
//     username: "john_doe",
//     role: "owner",
//     joined_at: "2025-09-30T12:00:00Z"
//   }
// ]
```

### Add a Member to Project (Owner Only)

```typescript
import { addProjectMember } from "@/lib/api/project-members";

const newMember = await addProjectMember(projectId, {
  user_id: "user-789",
  role: "member" // optional, defaults to "member"
});
```

**Roles:**
- `owner` - Full control (can add/remove members, transfer ownership)
- `member` - Can create conversations and access public conversations

### Remove a Member (Owner Only)

```typescript
import { removeProjectMember } from "@/lib/api/project-members";

await removeProjectMember(projectId, userId);
// User must transfer or delete their conversations first
// Cannot remove the last owner
```

### Update Member Role (Owner Only)

```typescript
import { updateMemberRole } from "@/lib/api/project-members";

await updateMemberRole(projectId, userId, {
  role: "owner" // or "member"
});
// Cannot demote the last owner
```

### Transfer Project Ownership (Owner Only)

```typescript
import { transferProjectOwnership } from "@/lib/api/project-members";

await transferProjectOwnership(projectId, {
  new_owner_user_id: "user-456"
});
// New owner must already be a member
// Current owner is automatically demoted to "member"
```

## Conversation Visibility API

### Check Conversation Visibility

```typescript
import { chatStore } from "@/lib/store/chat/chat-store";

const conversation = chatStore.map[conversationId];
const visibility = conversation.visibility; // "private" | "public" | undefined
const createdBy = conversation.created_by_user_id;

// Check if current user can see this conversation
const isPublic = visibility === "public";
const isOwner = createdBy === currentUserId;
const canView = isPublic || isOwner;
```

### Toggle Conversation Visibility

```typescript
import { toggleConversationVisibility } from "@/lib/api/conversations";

// Toggle between private and public
const result = await toggleConversationVisibility(conversationId);
console.log(result.visibility); // "private" or "public"

// Only the creator or project owner can toggle visibility
```

## Access Control Rules

### Projects
- Users can only see projects where they are members (in `project_members` table)
- Root users can see all projects

### Conversations
- **Public conversations**: Visible to all project members
- **Private conversations**: Only visible to the creator
- Root users can see all conversations

### Permissions
- **Add/remove members**: Project owners only
- **Transfer ownership**: Project owners only
- **Toggle conversation visibility**: Creator or project owner
- **Create conversations**: All project members

## UI Integration Examples

### Display Visibility Badge

```tsx
import { Lock, Globe } from "lucide-react";

function ConversationItem({ conversation }) {
  const isPrivate = conversation.visibility === "private";

  return (
    <div>
      {conversation.title}
      {isPrivate ? (
        <Lock className="h-4 w-4" />
      ) : (
        <Globe className="h-4 w-4" />
      )}
    </div>
  );
}
```

### Toggle Visibility Button

```tsx
import { toggleConversationVisibility } from "@/lib/api/conversations";
import { chatStore } from "@/lib/store/chat/chat-store";

function VisibilityToggle({ conversationId }) {
  const conversation = chatStore.map[conversationId];
  const isPrivate = conversation.visibility === "private";

  const handleToggle = async () => {
    const result = await toggleConversationVisibility(conversationId);
    // Update local store
    conversation.visibility = result.visibility;
  };

  return (
    <button onClick={handleToggle}>
      {isPrivate ? "Make Public" : "Make Private"}
    </button>
  );
}
```

### Project Members Management

```tsx
import {
  listProjectMembers,
  addProjectMember,
  removeProjectMember
} from "@/lib/api/project-members";

function ProjectMembersDialog({ projectId }) {
  const [members, setMembers] = useState([]);

  useEffect(() => {
    listProjectMembers(projectId).then(setMembers);
  }, [projectId]);

  const handleAddMember = async (userId: string) => {
    await addProjectMember(projectId, { user_id: userId });
    // Refresh members list
    const updated = await listProjectMembers(projectId);
    setMembers(updated);
  };

  const handleRemoveMember = async (userId: string) => {
    await removeProjectMember(projectId, userId);
    // Refresh members list
    const updated = await listProjectMembers(projectId);
    setMembers(updated);
  };

  return (
    <div>
      <h2>Project Members</h2>
      {members.map(member => (
        <div key={member.id}>
          <span>{member.username}</span>
          <span>{member.role}</span>
          <button onClick={() => handleRemoveMember(member.user_id)}>
            Remove
          </button>
        </div>
      ))}
    </div>
  );
}
```

## Database Schema

### project_members table
```sql
CREATE TABLE project_members (
    id UUID PRIMARY KEY,
    project_id VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL,
    role VARCHAR(50) CHECK (role IN ('owner', 'member')),
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);
```

### conversations additions
```sql
ALTER TABLE conversations
    ADD COLUMN created_by_user_id UUID,
    ADD COLUMN visibility VARCHAR(50) DEFAULT 'private'
        CHECK (visibility IN ('private', 'public'));
```

## Migration

The migration file `20250930_add_multi_user_support.sql` handles:
1. Creating `project_members` table
2. Adding visibility fields to conversations
3. Backfilling existing data (current project owners → project_members)
4. Setting existing conversations to private with current owner as creator

## Error Handling

```typescript
try {
  await addProjectMember(projectId, { user_id: userId });
} catch (error) {
  // Handle errors:
  // - 403: Not a project owner
  // - 404: User not found
  // - 400: User already a member
  console.error("Failed to add member:", error);
}

try {
  await removeProjectMember(projectId, userId);
} catch (error) {
  // Handle errors:
  // - 403: Not a project owner
  // - 400: Cannot remove last owner, or user has conversations
  console.error("Failed to remove member:", error);
}

try {
  await toggleConversationVisibility(conversationId);
} catch (error) {
  // Handle errors:
  // - 403: Not creator or project owner
  // - 404: Conversation not found
  console.error("Failed to toggle visibility:", error);
}
```

## Best Practices

1. **Check permissions before showing UI**: Hide add/remove buttons for non-owners
2. **Refresh member list after changes**: Always re-fetch after add/remove operations
3. **Show visibility indicators**: Make it clear which conversations are private vs public
4. **Confirm ownership transfers**: Show confirmation dialog (irreversible action)
5. **Handle errors gracefully**: Show user-friendly error messages

## Next Steps

To implement the UI:
1. Create a ProjectMembersDialog component
2. Add visibility toggle button to conversation header
3. Add visibility icons to conversation list items
4. Update conversation filtering to respect visibility rules