import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Copy, ExternalLink, Code, Palette, Settings } from "lucide-react";
import { toast } from "sonner";

// This is a demo page to show how the embedding works
export function EmbedDemo() {
  const [shareToken, setShareToken] = useState("share_demo123example");
  const [embedType, setEmbedType] = useState("widget");
  const [theme, setTheme] = useState("light");
  const [layout, setLayout] = useState("combined");
  const [readOnly, setReadOnly] = useState(false);
  const [width, setWidth] = useState("400");
  const [height, setHeight] = useState("600");

  const baseUrl = window.location.origin;
  const embedUrl = `${baseUrl}/embed/${shareToken}?type=${embedType}&theme=${theme}&layout=${layout}${readOnly ? '&readonly=true' : ''}`;

  const iframeCode = `<iframe 
  src="${embedUrl}"
  width="${width}px" 
  height="${height}px"
  frameborder="0"
  style="border-radius: 8px;">
</iframe>`;

  const responsiveIframeCode = `<div style="position: relative; padding-bottom: 75%; height: 0; border-radius: 8px; overflow: hidden;">
  <iframe 
    src="${embedUrl}"
    style="position: absolute; top: 0; left: 0; width: 100%; height: 100%;"
    frameborder="0">
  </iframe>
</div>`;

  const javascriptCode = `<div id="clay-chat"></div>
<script src="${baseUrl}/embed.js"></script>
<script>
  ClayStudio.embed({
    token: '${shareToken}',
    container: '#clay-chat',
    type: '${embedType}',
    theme: '${theme}',
    layout: '${layout}',
    ${readOnly ? 'readOnly: true,' : ''}
    width: '${width}px',
    height: '${height}px',
    onMessage: (message) => console.log('New message:', message),
    onError: (error) => console.error('Error:', error)
  });
</script>`;

  const dataAttributeCode = `<!-- Auto-initialized with data attributes -->
<div 
  data-clay-studio="${shareToken}"
  data-type="${embedType}"
  data-theme="${theme}"
  data-layout="${layout}"
  ${readOnly ? 'data-readonly="true"' : ''}
  data-width="${width}px"
  data-height="${height}px">
</div>
<script src="${baseUrl}/embed.js"></script>`;

  const reactCode = `import { ClayChat } from '@clay-studio/embed-react';

function App() {
  return (
    <ClayChat 
      shareToken="${shareToken}"
      type="${embedType}"
      theme="${theme}"
      layout="${layout}"
      ${readOnly ? 'readOnly={true}' : ''}
      width="${width}px"
      height="${height}px"
      onMessage={(msg) => console.log('New message:', msg)}
    />
  );
}`;

  const copyToClipboard = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`${label} copied to clipboard!`);
    } catch (error) {
      toast.error("Failed to copy to clipboard");
    }
  };

  return (
    <div className="container mx-auto py-8 px-4 max-w-7xl">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Clay Studio Embed Demo</h1>
        <p className="text-muted-foreground">
          See how shared projects can be embedded into websites and applications.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Configuration Panel */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Settings className="w-5 h-5" />
                Configuration
              </CardTitle>
              <CardDescription>
                Customize how your shared chat appears when embedded.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Share Token</Label>
                <Input
                  value={shareToken}
                  onChange={(e) => setShareToken(e.target.value)}
                  placeholder="share_abc123"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Type</Label>
                  <Select value={embedType} onValueChange={setEmbedType}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="widget">Full Widget</SelectItem>
                      <SelectItem value="chat">Chat Only</SelectItem>
                      <SelectItem value="list">Conversations Only</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label>Theme</Label>
                  <Select value={theme} onValueChange={setTheme}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="light">Light</SelectItem>
                      <SelectItem value="dark">Dark</SelectItem>
                      <SelectItem value="auto">Auto</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Layout</Label>
                  <Select value={layout} onValueChange={setLayout}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="combined">Combined</SelectItem>
                      <SelectItem value="chat-only">Chat Only</SelectItem>
                      <SelectItem value="list-only">List Only</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label>Mode</Label>
                  <Select value={readOnly ? "readonly" : "interactive"} onValueChange={(v) => setReadOnly(v === "readonly")}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="interactive">Interactive</SelectItem>
                      <SelectItem value="readonly">Read Only</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Width (px)</Label>
                  <Input
                    value={width}
                    onChange={(e) => setWidth(e.target.value)}
                    type="number"
                  />
                </div>

                <div className="space-y-2">
                  <Label>Height (px)</Label>
                  <Input
                    value={height}
                    onChange={(e) => setHeight(e.target.value)}
                    type="number"
                  />
                </div>
              </div>

              <div className="flex items-center justify-between pt-4 border-t">
                <div className="space-y-1">
                  <div className="text-sm font-medium">Preview URL</div>
                  <div className="text-xs text-muted-foreground">
                    {embedUrl}
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => window.open(embedUrl, '_blank')}
                >
                  <ExternalLink className="w-4 h-4 mr-1" />
                  Open
                </Button>
              </div>
            </CardContent>
          </Card>

          {/* Live Preview */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Palette className="w-5 h-5" />
                Live Preview
              </CardTitle>
              <CardDescription>
                See how your embedded chat will look.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="border rounded-lg overflow-hidden">
                <iframe
                  src={embedUrl}
                  width="100%"
                  height="400"
                  frameBorder="0"
                  style={{ borderRadius: '8px' }}
                />
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Code Examples */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Code className="w-5 h-5" />
                Embed Codes
              </CardTitle>
              <CardDescription>
                Copy and paste these code snippets to embed your shared chat.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Tabs defaultValue="iframe" className="w-full">
                <TabsList className="grid grid-cols-4 w-full">
                  <TabsTrigger value="iframe" className="text-xs">iframe</TabsTrigger>
                  <TabsTrigger value="responsive" className="text-xs">Responsive</TabsTrigger>
                  <TabsTrigger value="javascript" className="text-xs">JavaScript</TabsTrigger>
                  <TabsTrigger value="react" className="text-xs">React</TabsTrigger>
                </TabsList>

                <TabsContent value="iframe" className="space-y-3">
                  <div className="flex items-center justify-between">
                    <Badge variant="secondary">Simple iframe</Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => copyToClipboard(iframeCode, "iframe code")}
                    >
                      <Copy className="w-4 h-4 mr-1" />
                      Copy
                    </Button>
                  </div>
                  <pre className="text-sm bg-muted p-3 rounded-md overflow-x-auto">
                    <code>{iframeCode}</code>
                  </pre>
                </TabsContent>

                <TabsContent value="responsive" className="space-y-3">
                  <div className="flex items-center justify-between">
                    <Badge variant="secondary">Responsive iframe</Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => copyToClipboard(responsiveIframeCode, "responsive iframe code")}
                    >
                      <Copy className="w-4 h-4 mr-1" />
                      Copy
                    </Button>
                  </div>
                  <pre className="text-sm bg-muted p-3 rounded-md overflow-x-auto">
                    <code>{responsiveIframeCode}</code>
                  </pre>
                </TabsContent>

                <TabsContent value="javascript" className="space-y-3">
                  <div className="flex items-center justify-between">
                    <Badge variant="secondary">JavaScript SDK</Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => copyToClipboard(javascriptCode, "JavaScript code")}
                    >
                      <Copy className="w-4 h-4 mr-1" />
                      Copy
                    </Button>
                  </div>
                  <pre className="text-sm bg-muted p-3 rounded-md overflow-x-auto">
                    <code>{javascriptCode}</code>
                  </pre>

                  <div className="mt-4">
                    <h4 className="text-sm font-medium mb-2">Alternative: Data Attributes</h4>
                    <pre className="text-sm bg-muted p-3 rounded-md overflow-x-auto">
                      <code>{dataAttributeCode}</code>
                    </pre>
                  </div>
                </TabsContent>

                <TabsContent value="react" className="space-y-3">
                  <div className="flex items-center justify-between">
                    <Badge variant="secondary">React Component</Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => copyToClipboard(reactCode, "React code")}
                    >
                      <Copy className="w-4 h-4 mr-1" />
                      Copy
                    </Button>
                  </div>
                  <pre className="text-sm bg-muted p-3 rounded-md overflow-x-auto">
                    <code>{reactCode}</code>
                  </pre>
                  <div className="text-xs text-muted-foreground mt-2">
                    Install: <code className="bg-muted px-1 rounded">npm install @clay-studio/embed-react</code>
                  </div>
                </TabsContent>
              </Tabs>
            </CardContent>
          </Card>

          {/* Features */}
          <Card>
            <CardHeader>
              <CardTitle>Features</CardTitle>
              <CardDescription>
                What's included in the embeddable chat widget.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Real-time messaging
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Conversation history
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Customizable themes
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Mobile responsive
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Read-only mode
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Multiple layouts
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Event callbacks
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  Secure sharing
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}