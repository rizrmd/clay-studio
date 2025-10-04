import { api } from "@/lib/utils/api";

export interface ProjectMember {
  id: string;
  project_id: string;
  user_id: string;
  username: string;
  role: "owner" | "member";
  joined_at: string;
}

export interface AddMemberRequest {
  user_id: string;
  role?: "owner" | "member";
}

export interface UpdateMemberRoleRequest {
  role: "owner" | "member";
}

export interface TransferOwnershipRequest {
  new_owner_user_id: string;
}

/**
 * List all members of a project
 */
export async function listProjectMembers(
  projectId: string
): Promise<ProjectMember[]> {
  return api.get(`/projects/${projectId}/members`);
}

/**
 * Add a new member to a project (owner only)
 */
export async function addProjectMember(
  projectId: string,
  request: AddMemberRequest
): Promise<ProjectMember> {
  return api.post(`/projects/${projectId}/members`, request);
}

/**
 * Remove a member from a project (owner only)
 */
export async function removeProjectMember(
  projectId: string,
  userId: string
): Promise<{ message: string; user_id: string }> {
  return api.delete(`/projects/${projectId}/members/${userId}`);
}

/**
 * Update a member's role (owner only)
 */
export async function updateMemberRole(
  projectId: string,
  userId: string,
  request: UpdateMemberRoleRequest
): Promise<{ message: string; user_id: string; new_role: string }> {
  return api.patch(`/projects/${projectId}/members/${userId}`, request);
}

/**
 * Transfer project ownership to another member (owner only)
 */
export async function transferProjectOwnership(
  projectId: string,
  request: TransferOwnershipRequest
): Promise<{ message: string; new_owner_user_id: string }> {
  return api.post(`/projects/${projectId}/transfer`, request);
}