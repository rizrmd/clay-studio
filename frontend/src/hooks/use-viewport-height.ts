import { useEffect, useRef } from 'react';
import { useSnapshot } from 'valtio';
import { store, setViewportHeight, setKeyboardHeight } from '@/store/chat-store';

/**
 * Hook to monitor viewport height changes and detect virtual keyboard
 * Uses ResizeObserver and visualViewport API to detect keyboard on mobile
 */
export function useViewportHeight() {
  const snapshot = useSnapshot(store);
  const initialHeight = useRef<number>(0);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);

  useEffect(() => {
    // Store initial viewport height
    initialHeight.current = window.innerHeight;
    setViewportHeight(window.innerHeight);

    // Function to calculate keyboard height
    const updateKeyboardHeight = () => {
      let currentHeight = window.innerHeight;
      
      // Use visualViewport if available (better for mobile)
      if (window.visualViewport) {
        currentHeight = window.visualViewport.height;
      }

      const keyboardHeight = Math.max(0, initialHeight.current - currentHeight);
      
      // Only update if there's a significant change (>50px) to avoid false positives
      if (Math.abs(store.ui.keyboardHeight - keyboardHeight) > 50) {
        setKeyboardHeight(keyboardHeight);
        setViewportHeight(currentHeight);
      } else if (keyboardHeight < 50 && store.ui.keyboardHeight > 0) {
        // Keyboard was closed
        setKeyboardHeight(0);
        setViewportHeight(initialHeight.current);
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
      // Reset initial height on orientation change
      setTimeout(() => {
        initialHeight.current = window.innerHeight;
        setViewportHeight(window.innerHeight);
        setKeyboardHeight(0);
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
    viewportHeight: snapshot.ui.viewportHeight,
    keyboardHeight: snapshot.ui.keyboardHeight,
    isKeyboardOpen: snapshot.ui.keyboardHeight > 0,
  };
}