import { proxy } from 'valtio';

interface FileUpload {
  id: string;
  file_name: string;
  original_name: string;
  file_path: string;
  size: number;
  mime_type?: string;
  description?: string;
  auto_description?: string;
  created_at: string;
  is_text_file: boolean;
  preview?: string;
}

interface FileManagerState {
  files: FileUpload[];
  loading: boolean;
  editingFile: string | null;
  editDescription: string;
  savingDescription: boolean;
}

export const fileManagerStore = proxy<FileManagerState>({
  files: [],
  loading: true,
  editingFile: null,
  editDescription: '',
  savingDescription: false,
});

export const fileManagerActions = {
  setFiles: (files: FileUpload[]) => {
    fileManagerStore.files = files;
  },

  setLoading: (loading: boolean) => {
    fileManagerStore.loading = loading;
  },

  setEditingFile: (fileId: string | null) => {
    fileManagerStore.editingFile = fileId;
  },

  setEditDescription: (description: string) => {
    fileManagerStore.editDescription = description;
  },

  setSavingDescription: (saving: boolean) => {
    fileManagerStore.savingDescription = saving;
  },

  startEditDescription: (file: FileUpload) => {
    fileManagerStore.editingFile = file.id;
    fileManagerStore.editDescription = file.description || file.auto_description || '';
  },

  cancelEdit: () => {
    fileManagerStore.editingFile = null;
    fileManagerStore.editDescription = '';
  },

  updateFile: (fileId: string, updatedFile: FileUpload) => {
    fileManagerStore.files = fileManagerStore.files.map(f =>
      f.id === fileId ? updatedFile : f
    );
  },
};