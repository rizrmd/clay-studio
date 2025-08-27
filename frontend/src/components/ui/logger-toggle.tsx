import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { logger, LogLevel } from '@/lib/logger';

export function LoggerToggle() {
  const [config, setConfig] = useState(logger.getConfig());

  useEffect(() => {
    // Update local state when config changes
    setConfig(logger.getConfig());
  }, []);

  const handleToggle = () => {
    const newEnabled = logger.toggle();
    setConfig({ ...config, enabled: newEnabled });
  };

  const handleLevelChange = (level: LogLevel) => {
    logger.setLevel(level);
    setConfig({ ...config, level });
  };

  return (
    <Card className="w-full max-w-md">
      <CardHeader>
        <CardTitle className="flex items-center justify-between">
          Debug Logging
          <Badge variant={config.enabled ? "default" : "secondary"}>
            {config.enabled ? "Enabled" : "Disabled"}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">Enable Logging</span>
          <Button
            variant={config.enabled ? "default" : "outline"}
            size="sm"
            onClick={handleToggle}
          >
            {config.enabled ? "Disable" : "Enable"}
          </Button>
        </div>
        
        {config.enabled && (
          <div className="space-y-2">
            <label className="text-sm font-medium">Log Level</label>
            <Select value={config.level} onValueChange={handleLevelChange}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="debug">Debug</SelectItem>
                <SelectItem value="info">Info</SelectItem>
                <SelectItem value="warn">Warning</SelectItem>
                <SelectItem value="error">Error</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              Higher levels include all lower level messages
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}