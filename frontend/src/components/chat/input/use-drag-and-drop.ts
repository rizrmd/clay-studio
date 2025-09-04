import { useRef, useEffect } from "react";
import { inputActions } from "@/lib/store/chat/input-store";

export function useDragAndDrop(onFilesDropped: (files: File[]) => void) {
  const dragCounter = useRef(0);

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current++;
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      inputActions.setDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current = Math.max(0, dragCounter.current - 1);
    if (dragCounter.current === 0) {
      inputActions.setDragging(false);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    inputActions.setDragging(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const droppedFiles = Array.from(e.dataTransfer.files);
      onFilesDropped(droppedFiles);
      e.dataTransfer.clearData();
    }
  };

  // Cleanup effect for drag state
  useEffect(() => {
    const handleGlobalDragEnd = () => {
      inputActions.setDragging(false);
      dragCounter.current = 0;
    };

    const handleWindowBlur = () => {
      inputActions.setDragging(false);
      dragCounter.current = 0;
    };

    window.addEventListener('blur', handleWindowBlur);
    window.addEventListener('dragend', handleGlobalDragEnd);
    document.addEventListener('dragend', handleGlobalDragEnd);

    return () => {
      window.removeEventListener('blur', handleWindowBlur);
      window.removeEventListener('dragend', handleGlobalDragEnd);
      document.removeEventListener('dragend', handleGlobalDragEnd);
    };
  }, []);

  return {
    handleDragEnter,
    handleDragLeave,
    handleDragOver,
    handleDrop,
  };
}