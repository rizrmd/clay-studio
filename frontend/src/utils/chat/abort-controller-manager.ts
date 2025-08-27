class AbortControllerManager {
  private controllers: Map<string, AbortController> = new Map();

  create(key: string): AbortController {
    // Abort existing controller if any
    this.abort(key);
    
    const controller = new AbortController();
    this.controllers.set(key, controller);
    return controller;
  }

  get(key: string): AbortController | null {
    return this.controllers.get(key) || null;
  }

  abort(key: string): void {
    const controller = this.controllers.get(key);
    if (controller) {
      controller.abort();
      this.controllers.delete(key);
    }
  }

  abortAll(): void {
    for (const [, controller] of this.controllers) {
      controller.abort();
    }
    this.controllers.clear();
  }

  has(key: string): boolean {
    return this.controllers.has(key);
  }

  clear(): void {
    this.controllers.clear();
  }

  transfer(fromKey: string, toKey: string): void {
    const controller = this.controllers.get(fromKey);
    if (controller) {
      this.controllers.delete(fromKey);
      this.controllers.set(toKey, controller);
    }
  }
}

export const abortControllerManager = new AbortControllerManager();