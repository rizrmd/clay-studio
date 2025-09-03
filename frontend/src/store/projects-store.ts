import { proxy } from 'valtio';

interface Project {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  client_id: string;
  conversation_count?: number;
  datasource_count?: number;
}

interface ProjectsState {
  projects: Project[];
  isCreating: boolean;
  newProjectName: string;
  loading: boolean;
  error: string | null;
  deleteDialogOpen: boolean;
  projectToDelete: Project | null;
  isDeleting: boolean;
}

export const projectsStore = proxy<ProjectsState>({
  projects: [],
  isCreating: false,
  newProjectName: '',
  loading: false,
  error: null,
  deleteDialogOpen: false,
  projectToDelete: null,
  isDeleting: false,
});

export const projectsActions = {
  // Projects management
  setProjects: (projects: Project[]) => {
    projectsStore.projects = projects;
  },

  addProject: (project: Project) => {
    projectsStore.projects = [...projectsStore.projects, project];
  },

  removeProject: (projectId: string) => {
    projectsStore.projects = projectsStore.projects.filter(p => p.id !== projectId);
  },

  // Creation state
  setIsCreating: (isCreating: boolean) => {
    projectsStore.isCreating = isCreating;
  },

  setNewProjectName: (name: string) => {
    projectsStore.newProjectName = name;
  },

  clearNewProjectForm: () => {
    projectsStore.newProjectName = '';
    projectsStore.isCreating = false;
  },

  // Loading and error states
  setLoading: (loading: boolean) => {
    projectsStore.loading = loading;
  },

  setError: (error: string | null) => {
    projectsStore.error = error;
  },

  // Delete dialog state
  setDeleteDialogOpen: (open: boolean) => {
    projectsStore.deleteDialogOpen = open;
  },

  setProjectToDelete: (project: Project | null) => {
    projectsStore.projectToDelete = project;
  },

  clearDeleteState: () => {
    projectsStore.deleteDialogOpen = false;
    projectsStore.projectToDelete = null;
  },

  // Delete loading state
  setIsDeleting: (isDeleting: boolean) => {
    projectsStore.isDeleting = isDeleting;
  },

  // Reset all state
  reset: () => {
    projectsStore.projects = [];
    projectsStore.isCreating = false;
    projectsStore.newProjectName = '';
    projectsStore.loading = false;
    projectsStore.error = null;
    projectsStore.deleteDialogOpen = false;
    projectsStore.projectToDelete = null;
    projectsStore.isDeleting = false;
  },
};