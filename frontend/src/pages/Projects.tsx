import { useEffect } from "react";
import { useNavigate, Link } from "react-router-dom";
import { useSnapshot } from "valtio";
import {
  Plus,
  FolderOpen,
  Calendar,
  ChevronRight,
  MessageSquare,
  Database,
  Trash2,
  MoreVertical,
} from "lucide-react";
import { api } from "@/lib/utils/api";
import { useAuth } from "@/hooks/use-auth";
import { projectsStore, projectsActions } from "@/store/projects-store";
import { ProjectsSkeleton } from "@/components/projects/projects-skeleton";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface Project {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  client_id: string;
  conversation_count?: number;
  datasource_count?: number;
}

export function ProjectsPage() {
  const projectsSnapshot = useSnapshot(projectsStore);
  const navigate = useNavigate();
  const {} = useAuth();

  useEffect(() => {
    fetchProjects();
  }, []);

  const fetchProjects = async () => {
    projectsActions.setLoading(true);
    projectsActions.setError(null);
    try {
      const data = await api.get<Project[]>("/projects");
      projectsActions.setProjects(data);
    } catch (err) {
      projectsActions.setError(err instanceof Error ? err.message : "Failed to load projects");
    } finally {
      projectsActions.setLoading(false);
    }
  };

  const createProject = async () => {
    if (!projectsSnapshot.newProjectName.trim()) return;

    projectsActions.setLoading(true);
    projectsActions.setError(null);
    try {
      const newProject = await api.post<Project>("/projects", {
        name: projectsSnapshot.newProjectName,
      });

      projectsActions.addProject(newProject);
      projectsActions.clearNewProjectForm();

      // Navigate to chat with the new project
      navigate(`/p/${newProject.id}/new`);
    } catch (err) {
      projectsActions.setError(err instanceof Error ? err.message : "Failed to create project");
    } finally {
      projectsActions.setLoading(false);
    }
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  const handleDeleteClick = (e: React.MouseEvent, project: Project) => {
    e.preventDefault();
    e.stopPropagation();
    projectsActions.setProjectToDelete(project);
    projectsActions.setDeleteDialogOpen(true);
  };

  const deleteProject = async () => {
    if (!projectsSnapshot.projectToDelete) return;

    projectsActions.setIsDeleting(true);
    try {
      await api.delete(`/projects/${projectsSnapshot.projectToDelete.id}`);
      projectsActions.removeProject(projectsSnapshot.projectToDelete.id);
      projectsActions.clearDeleteState();
    } catch (err) {
      projectsActions.setError(err instanceof Error ? err.message : "Failed to delete project");
    } finally {
      projectsActions.setIsDeleting(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100">
            Your Projects
          </h1>
          <p className="mt-2 text-gray-600 dark:text-gray-400">
            Select a project to start chatting with Clay Studio or create a new
            one
          </p>
        </div>

        {/* Error Message */}
        {projectsSnapshot.error && (
          <div className="mb-6 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
            <p className="text-sm text-red-800 dark:text-red-300">{projectsSnapshot.error}</p>
          </div>
        )}

        {/* Loading State */}
        {projectsSnapshot.loading && projectsSnapshot.projects.length === 0 ? (
          <ProjectsSkeleton />
        ) : (
          <>
            {/* Projects Grid */}
            {projectsSnapshot.projects.length > 0 ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-6">
                {projectsSnapshot.projects.map((project) => (
                  <div
                    key={project.id}
                    className="relative bg-white dark:bg-gray-800 rounded-lg shadow-sm hover:shadow-md transition-shadow border border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-600 group"
                  >
                    <Link
                      to={`/p/${project.id}`}
                      className="block p-6"
                    >
                      <div className="flex items-start justify-between mb-4">
                        <div className="flex items-center">
                          <FolderOpen className="h-8 w-8 text-blue-600 dark:text-blue-400" />
                        </div>
                        <div className="flex items-center gap-2">
                          <ChevronRight className="h-5 w-5 text-gray-400" />
                        </div>
                      </div>

                      <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
                        {project.name}
                      </h3>

                      <div className="flex items-center text-sm text-gray-500 dark:text-gray-400 mb-3">
                        <Calendar className="h-4 w-4 mr-1" />
                        <span>Created {formatDate(project.created_at)}</span>
                      </div>

                      {/* Project Stats */}
                      <div className="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-300">
                        <div className="flex items-center gap-1">
                          <MessageSquare className="h-4 w-4" />
                          <span>{project.conversation_count || 0} chats</span>
                        </div>
                        <div className="flex items-center gap-1">
                          <Database className="h-4 w-4" />
                          <span>
                            {project.datasource_count || 0} datasources
                          </span>
                        </div>
                      </div>
                    </Link>
                    {/* Project Actions Dropdown */}
                    <div className="absolute bottom-4 right-4">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <button
                            onClick={(e) => e.preventDefault()}
                            className="p-1 rounded-md opacity-0 group-hover:opacity-100 hover:bg-gray-100 dark:hover:bg-gray-700 transition-opacity"
                          >
                            <MoreVertical className="h-5 w-5 text-gray-600 dark:text-gray-400" />
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem
                            onClick={(e) => handleDeleteClick(e, project)}
                            className="text-red-600 dark:text-red-400"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete Project
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </div>
                ))}

                {/* Create New Project Card */}
                {!projectsSnapshot.isCreating ? (
                  <button
                    onClick={() => projectsActions.setIsCreating(true)}
                    type="button"
                    className="bg-white dark:bg-gray-800 rounded-lg shadow-sm hover:shadow-md transition-shadow cursor-pointer border-2 border-dashed border-gray-300 dark:border-gray-600 hover:border-blue-400 dark:hover:border-blue-500 w-full text-left"
                  >
                    <div className="p-6 flex flex-col items-center justify-center h-full min-h-[200px]">
                      <Plus className="h-12 w-12 text-gray-400 dark:text-gray-500 mb-4" />
                      <span className="text-lg font-medium text-gray-600 dark:text-gray-400">
                        Create New Project
                      </span>
                    </div>
                  </button>
                ) : (
                  <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-blue-300 dark:border-blue-600">
                    <div className="p-6">
                      <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
                        New Project
                      </h3>
                       <input
                         type="text"
                         value={projectsSnapshot.newProjectName}
                         onChange={(e) => projectsActions.setNewProjectName(e.target.value)}
                         onKeyPress={(e) => e.key === "Enter" && createProject()}
                         placeholder="Enter project name"
                         className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 mb-4"
                         autoFocus
                       />
                       <div className="flex gap-2">
                         <button
                           onClick={createProject}
                           disabled={projectsSnapshot.loading || !projectsSnapshot.newProjectName.trim()}
                           className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                         >
                           Create
                         </button>
                         <button
                           onClick={() => projectsActions.clearNewProjectForm()}
                           className="flex-1 px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-md hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                         >
                           Cancel
                         </button>
                       </div>
                    </div>
                  </div>
                )}
              </div>
            ) : (
              /* Empty State */
              <div className="text-center py-12 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                <FolderOpen className="h-16 w-16 text-gray-400 dark:text-gray-500 mx-auto mb-4" />
                <h3 className="text-xl font-medium text-gray-900 dark:text-gray-100 mb-2">
                  No projects yet
                </h3>
                <p className="text-gray-600 dark:text-gray-400 mb-6">
                  Create your first project to start using Claude
                </p>
                 {!projectsSnapshot.isCreating ? (
                   <button
                     onClick={() => projectsActions.setIsCreating(true)}
                     className="inline-flex items-center gap-2 px-6 py-3 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
                   >
                     <Plus className="h-5 w-5" />
                     Create Your First Project
                   </button>
                 ) : (
                   <div className="max-w-md mx-auto">
                     <input
                       type="text"
                       value={projectsSnapshot.newProjectName}
                       onChange={(e) => projectsActions.setNewProjectName(e.target.value)}
                       onKeyPress={(e) => e.key === "Enter" && createProject()}
                       placeholder="Enter project name"
                       className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 mb-4"
                       autoFocus
                     />
                     <div className="flex gap-2">
                       <button
                         onClick={createProject}
                         disabled={projectsSnapshot.loading || !projectsSnapshot.newProjectName.trim()}
                         className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                       >
                         Create Project
                       </button>
                       <button
                         onClick={() => projectsActions.clearNewProjectForm()}
                         className="flex-1 px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-md hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                       >
                         Cancel
                       </button>
                     </div>
                   </div>
                 )}
              </div>
            )}
          </>
        )}
      </div>

      {/* Delete Confirmation Dialog */}
      <Dialog open={projectsSnapshot.deleteDialogOpen} onOpenChange={projectsActions.setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Project</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{projectsSnapshot.projectToDelete?.name}"? This action cannot be undone.
              All conversations and data sources associated with this project will be removed.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => projectsActions.setDeleteDialogOpen(false)}
              disabled={projectsSnapshot.isDeleting}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={deleteProject}
              disabled={projectsSnapshot.isDeleting}
            >
              {projectsSnapshot.isDeleting ? "Deleting..." : "Delete Project"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
