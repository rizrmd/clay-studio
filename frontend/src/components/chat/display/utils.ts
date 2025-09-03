import { API_BASE_URL } from "@/lib/utils/url";
import { FileAttachment } from "./types";

export const formatFileSize = (bytes: number) => {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + " " + sizes[i];
};

export const handleDownloadFile = async (file: FileAttachment) => {
  try {
    const clientId = localStorage.getItem("activeClientId");
    const projectId = localStorage.getItem("activeProjectId");
    if (!clientId || !projectId) return;

    const fileName = file.file_path.split("/").pop();
    const downloadUrl = `${API_BASE_URL}/uploads/${clientId}/${projectId}/${fileName}`;

    const link = document.createElement("a");
    link.href = downloadUrl;
    link.download = file.original_name || fileName || "";
    link.target = "_blank";
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  } catch (error) {
    // Error downloading file
  }
};