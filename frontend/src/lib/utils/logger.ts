type LogLevel = 'debug' | 'info' | 'warn' | 'error';

interface LoggerConfig {
  enabled: boolean;
  level: LogLevel;
  prefix: string;
}

class Logger {
  private config: LoggerConfig;
  
  constructor() {
    this.config = {
      enabled: this.getStoredValue('logger_enabled', true),
      level: this.getStoredValue('logger_level', 'debug') as LogLevel,
      prefix: '[Clay Studio]'
    };
  }

  private getStoredValue<T>(key: string, defaultValue: T): T {
    try {
      const stored = localStorage.getItem(key);
      return stored ? JSON.parse(stored) : defaultValue;
    } catch {
      return defaultValue;
    }
  }

  private setStoredValue(key: string, value: any): void {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // Silently fail if localStorage is not available
    }
  }

  private shouldLog(level: LogLevel): boolean {
    if (!this.config.enabled) return false;
    
    const levels: LogLevel[] = ['debug', 'info', 'warn', 'error'];
    const currentLevelIndex = levels.indexOf(this.config.level);
    const requestedLevelIndex = levels.indexOf(level);
    
    return requestedLevelIndex >= currentLevelIndex;
  }

  private formatMessage(level: LogLevel, message: string, ...args: any[]): [string, ...any[]] {
    const timestamp = new Date().toISOString().substring(11, 23);
    const levelStr = level.toUpperCase().padEnd(5);
    return [`${this.config.prefix} ${timestamp} [${levelStr}] ${message}`, ...args];
  }

  debug(message: string, ...args: any[]): void {
    this.refreshConfig();
    if (!this.config.enabled) return;
    if (this.shouldLog('debug')) {
      console.log(...this.formatMessage('debug', message, ...args));
    }
  }

  info(message: string, ...args: any[]): void {
    this.refreshConfig();
    if (!this.config.enabled) return;
    if (this.shouldLog('info')) {
      console.info(...this.formatMessage('info', message, ...args));
    }
  }

  warn(message: string, ...args: any[]): void {
    this.refreshConfig();
    if (!this.config.enabled) return;
    if (this.shouldLog('warn')) {
      console.warn(...this.formatMessage('warn', message, ...args));
    }
  }

  error(message: string, ...args: any[]): void {
    this.refreshConfig();
    if (!this.config.enabled) return;
    if (this.shouldLog('error')) {
      console.error(...this.formatMessage('error', message, ...args));
    }
  }

  // Configuration methods
  setEnabled(enabled: boolean): void {
    this.config.enabled = enabled;
    this.setStoredValue('logger_enabled', enabled);
  }

  setLevel(level: LogLevel): void {
    this.config.level = level;
    this.setStoredValue('logger_level', level);
  }

  getConfig(): LoggerConfig {
    // Always refresh from localStorage to catch external changes
    this.refreshConfig();
    return { ...this.config };
  }

  private refreshConfig(): void {
    this.config.enabled = this.getStoredValue('logger_enabled', true);
    this.config.level = this.getStoredValue('logger_level', 'debug') as LogLevel;
  }

  toggle(): boolean {
    this.refreshConfig(); // Ensure we have latest config
    const newEnabled = !this.config.enabled;
    this.setEnabled(newEnabled);
    return newEnabled;
  }
}

// Export singleton instance
export const logger = new Logger();

// Export types for external use
export type { LogLevel };