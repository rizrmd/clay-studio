// Clay Studio Embed SDK
// This file will be built as a standalone JavaScript library

interface EmbedOptions {
  token: string;
  container: string | HTMLElement;
  type?: 'chat' | 'list' | 'widget';
  theme?: 'light' | 'dark' | 'auto';
  readOnly?: boolean;
  layout?: 'combined' | 'chat-only' | 'list-only';
  width?: string;
  height?: string;
  features?: {
    showHeader?: boolean;
    showSidebar?: boolean;
    allowFileUpload?: boolean;
  };
  onMessage?: (message: any) => void;
  onError?: (error: string) => void;
  onReady?: () => void;
}

interface EmbedInstance {
  destroy: () => void;
  postMessage: (message: any) => void;
  updateOptions: (options: Partial<EmbedOptions>) => void;
}

class ClayStudioEmbed {
  private instances: Map<string, EmbedInstance> = new Map();

  embed(options: EmbedOptions): EmbedInstance {
    const {
      token,
      container,
      type = 'widget',
      theme = 'light',
      readOnly = false,
      layout = 'combined',
      width = '100%',
      height = '600px',
      features = {},
      onMessage,
      onError,
      onReady,
    } = options;

    // Get container element
    let containerElement: HTMLElement;
    if (typeof container === 'string') {
      const element = document.querySelector(container) as HTMLElement;
      if (!element) {
        throw new Error(`Container element not found: ${container}`);
      }
      containerElement = element;
    } else {
      containerElement = container;
    }

    // Create iframe for embedding
    const iframe = document.createElement('iframe');
    iframe.style.width = width;
    iframe.style.height = height;
    iframe.style.border = 'none';
    iframe.style.borderRadius = '8px';
    iframe.setAttribute('allow', 'clipboard-read; clipboard-write');

    // Build embed URL with parameters
    const baseUrl = this.getBaseUrl();
    const params = new URLSearchParams({
      type,
      theme,
      layout,
      ...(readOnly && { readonly: 'true' }),
      ...(features.showHeader === false && { header: 'false' }),
      ...(features.showSidebar === false && { sidebar: 'false' }),
      ...(features.allowFileUpload === false && { upload: 'false' }),
    });

    iframe.src = `${baseUrl}/embed/${token}?${params.toString()}`;

    // Add iframe to container
    containerElement.appendChild(iframe);

    // Set up message handling
    const messageHandler = (event: MessageEvent) => {
      if (event.origin !== baseUrl) return;

      const { type: msgType, data } = event.data;

      switch (msgType) {
        case 'clay-studio-message':
          onMessage?.(data);
          break;
        case 'clay-studio-error':
          onError?.(data);
          break;
        case 'clay-studio-ready':
          onReady?.();
          break;
        case 'clay-studio-conversation-click':
          // Handle conversation clicks from embedded view
          this.postMessageToIframe(iframe, {
            type: 'navigate-to-conversation',
            conversationId: data.conversationId,
          });
          break;
      }
    };

    window.addEventListener('message', messageHandler);

    // Create instance
    const instanceId = Math.random().toString(36).substr(2, 9);
    const instance: EmbedInstance = {
      destroy: () => {
        window.removeEventListener('message', messageHandler);
        containerElement.removeChild(iframe);
        this.instances.delete(instanceId);
      },

      postMessage: (message: any) => {
        this.postMessageToIframe(iframe, message);
      },

      updateOptions: (newOptions: Partial<EmbedOptions>) => {
        // Update iframe src with new options
        const currentUrl = new URL(iframe.src);
        const currentParams = new URLSearchParams(currentUrl.search);

        if (newOptions.theme) currentParams.set('theme', newOptions.theme);
        if (newOptions.layout) currentParams.set('layout', newOptions.layout);
        if (newOptions.readOnly !== undefined) {
          if (newOptions.readOnly) {
            currentParams.set('readonly', 'true');
          } else {
            currentParams.delete('readonly');
          }
        }

        currentUrl.search = currentParams.toString();
        iframe.src = currentUrl.toString();
      },
    };

    this.instances.set(instanceId, instance);
    return instance;
  }

  // Initialize multiple embeds from data attributes
  init(): void {
    const elements = document.querySelectorAll('[data-clay-studio]');
    elements.forEach((element) => {
      const htmlElement = element as HTMLElement;
      const token = htmlElement.dataset.clayStudio;
      if (!token) return;

      const options: EmbedOptions = {
        token,
        container: htmlElement,
        type: (htmlElement.dataset.type as any) || 'widget',
        theme: (htmlElement.dataset.theme as any) || 'light',
        readOnly: htmlElement.dataset.readonly === 'true',
        layout: (htmlElement.dataset.layout as any) || 'combined',
        width: htmlElement.dataset.width || '100%',
        height: htmlElement.dataset.height || '600px',
      };

      this.embed(options);
    });
  }

  private getBaseUrl(): string {
    // Try to detect the base URL from the script tag
    const scripts = document.querySelectorAll('script[src*="clay.studio"], script[src*="embed.js"]');
    if (scripts.length > 0) {
      const scriptSrc = (scripts[0] as HTMLScriptElement).src;
      const url = new URL(scriptSrc);
      return `${url.protocol}//${url.host}`;
    }

    // Fallback to production URL
    return 'https://clay.studio';
  }

  private postMessageToIframe(iframe: HTMLIFrameElement, message: any): void {
    if (iframe.contentWindow) {
      iframe.contentWindow.postMessage(message, this.getBaseUrl());
    }
  }
}

// Global instance
const ClayStudio = new ClayStudioEmbed();

// Auto-initialize on DOM ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => ClayStudio.init());
} else {
  ClayStudio.init();
}

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { ClayStudio };
}

if (typeof window !== 'undefined') {
  (window as any).ClayStudio = ClayStudio;
}

export { ClayStudio };
export type { EmbedOptions, EmbedInstance };