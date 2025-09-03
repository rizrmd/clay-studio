import { useEffect, useRef } from 'react';
import { useSnapshot } from 'valtio';
import { uiStore, uiActions } from '@/store/ui-store';

/**
 * Hook to monitor viewport height changes and detect virtual keyboard
 * Uses ResizeObserver and visualViewport API to detect keyboard on mobile
 */
export function useViewportHeight() {
  const snapshot = useSnapshot(uiStore);
  const maxHeight = useRef<number>(0);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);

  useEffect(() => {
    // Store initial viewport height and track maximum
    const currentHeight = window.innerHeight;
    maxHeight.current = currentHeight;
    uiActions.setViewportHeight(currentHeight);

    // Function to calculate keyboard height
    const updateKeyboardHeight = () => {
      let currentHeight = window.innerHeight;
      
      // Use visualViewport if available (better for mobile)
      if (window.visualViewport) {
        currentHeight = window.visualViewport.height;
      }

      // Update max height if current is larger (viewport expanded)
      if (currentHeight > maxHeight.current) {
        maxHeight.current = currentHeight;
      }

      // Calculate keyboard height based on max observed height
      const keyboardHeight = Math.max(0, maxHeight.current - currentHeight);
      
      // Always update viewport height to current value
      uiActions.setViewportHeight(currentHeight);
      
      // Only update keyboard height if there's a significant change (>50px)
      if (Math.abs(snapshot.keyboardHeight - keyboardHeight) > 50) {
        uiActions.setKeyboardHeight(keyboardHeight);
      } else if (keyboardHeight < 50 && snapshot.keyboardHeight > 0) {
        // Keyboard was closed
        uiActions.setKeyboardHeight(0);
      }
    };

    // Create ResizeObserver to monitor document height changes
    resizeObserverRef.current = new ResizeObserver(() => {
      updateKeyboardHeight();
    });

    // Observe the document element
    if (document.documentElement) {
      resizeObserverRef.current.observe(document.documentElement);
    }

    // Also listen to visualViewport resize if available
    const handleViewportChange = () => {
      updateKeyboardHeight();
    };

    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', handleViewportChange);
      window.visualViewport.addEventListener('scroll', handleViewportChange);
    }

    // Fallback to window resize
    window.addEventListener('resize', handleViewportChange);

    // Handle orientation changes
    const handleOrientationChange = () => {
      // Update max height on orientation change
      setTimeout(() => {
        const newHeight = window.innerHeight;
        maxHeight.current = newHeight;
        uiActions.setViewportHeight(newHeight);
        uiActions.setKeyboardHeight(0);
      }, 100);
    };

    window.addEventListener('orientationchange', handleOrientationChange);

    // Cleanup
    return () => {
      if (resizeObserverRef.current) {
        resizeObserverRef.current.disconnect();
      }
      if (window.visualViewport) {
        window.visualViewport.removeEventListener('resize', handleViewportChange);
        window.visualViewport.removeEventListener('scroll', handleViewportChange);
      }
      window.removeEventListener('resize', handleViewportChange);
      window.removeEventListener('orientationchange', handleOrientationChange);
    };
  }, []);

  return {
    viewportHeight: snapshot.viewportHeight,
    keyboardHeight: snapshot.keyboardHeight,
    isKeyboardOpen: snapshot.keyboardHeight > 0,
  };
}