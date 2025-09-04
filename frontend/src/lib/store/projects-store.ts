import { proxy } from 'valtio'

interface Project {
  id: string
  name: string
  created_at: string
  updated_at: string
  client_id: string
  conversation_count?: number
  datasource_count?: number
}

export const projectsStore = proxy({
  projects: [] as Project[],
  selectedProject: null as Project | null,
  isLoading: false,
  error: null as string | null,
  isCreating: false,
  isDeleting: false,
  newProjectName: '',
  deleteDialogOpen: false,
  projectToDelete: null as Project | null,
})

export const projectsActions = {
  setProjects: (projects: Project[]) => {
    projectsStore.projects = projects
  },
  
  addProject: (project: Project) => {
    projectsStore.projects.push(project)
  },
  
  updateProject: (id: string, updates: Partial<Project>) => {
    const index = projectsStore.projects.findIndex(p => p.id === id)
    if (index !== -1) {
      Object.assign(projectsStore.projects[index], updates)
    }
  },
  
  removeProject: (id: string) => {
    projectsStore.projects = projectsStore.projects.filter(p => p.id !== id)
  },
  
  setSelectedProject: (project: Project | null) => {
    projectsStore.selectedProject = project
  },
  
  setLoading: (isLoading: boolean) => {
    projectsStore.isLoading = isLoading
  },
  
  setError: (error: string | null) => {
    projectsStore.error = error
  },
  
  setCreating: (isCreating: boolean) => {
    projectsStore.isCreating = isCreating
  },
  
  setIsCreating: (isCreating: boolean) => {
    projectsStore.isCreating = isCreating
  },
  
  setDeleting: (isDeleting: boolean) => {
    projectsStore.isDeleting = isDeleting
  },
  
  setIsDeleting: (isDeleting: boolean) => {
    projectsStore.isDeleting = isDeleting
  },
  
  setSharing: (_isSharing: boolean) => {
    // Placeholder for sharing functionality
  },
  
  setNewProjectName: (name: string) => {
    projectsStore.newProjectName = name
  },
  
  clearNewProjectForm: () => {
    projectsStore.newProjectName = ''
    projectsStore.isCreating = false
  },
  
  setDeleteDialogOpen: (open: boolean) => {
    projectsStore.deleteDialogOpen = open
    if (!open) {
      projectsStore.projectToDelete = null
    }
  },
  
  setProjectToDelete: (project: Project | null) => {
    projectsStore.projectToDelete = project
  },
  
  clearDeleteState: () => {
    projectsStore.deleteDialogOpen = false
    projectsStore.projectToDelete = null
    projectsStore.isDeleting = false
  },
}