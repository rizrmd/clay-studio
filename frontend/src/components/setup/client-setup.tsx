"use client"

import React, { useState } from "react"
import { useForm } from "react-hook-form"
import { Plus, Settings, Trash2, Edit, Loader2, Check } from "lucide-react"
import axios from "@/lib/axios"

import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form"
import { Input } from "@/components/ui/input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Textarea } from "@/components/ui/textarea"
import { Badge } from "@/components/ui/badge"
import { Alert, AlertDescription } from "@/components/ui/alert"

interface Client {
  id: string
  name: string
  description?: string
  status: "pending" | "installing" | "active" | "error"
  installPath?: string
  createdAt: Date
}

interface ClientFormData {
  name: string
  description?: string
}

interface ClientSetupProps {
  onClientAdded?: () => void
}

export function ClientSetup({ onClientAdded }: ClientSetupProps = {}) {
  const [clients, setClients] = useState<Client[]>([])
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  const [editingClient, setEditingClient] = useState<Client | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  
  // Token setup state
  const [setupClient, setSetupClient] = useState<Client | null>(null)
  const [streamOutput, setStreamOutput] = useState<string[]>([])
  const [isSettingUpToken, setIsSettingUpToken] = useState(false)
  const [tokenSetupStep, setTokenSetupStep] = useState<"idle" | "streaming" | "completed" | "error">("idle")

  // Load existing clients on mount
  React.useEffect(() => {
    loadClients()
  }, [])

  // Poll for status updates for installing clients
  React.useEffect(() => {
    const interval = setInterval(() => {
      const hasInstallingClients = clients.some(c => c.status === "installing" || c.status === "pending")
      if (hasInstallingClients) {
        loadClients()
      }
    }, 3000) // Poll every 3 seconds

    return () => clearInterval(interval)
  }, [clients])

  const loadClients = async () => {
    try {
      const response = await axios.get('/clients')
      setClients(response.data)
    } catch (error) {
      // Failed to load clients
    }
  }

  const form = useForm<ClientFormData>({
    defaultValues: {
      name: "",
      description: "",
    },
  })

  const onSubmit = async (data: ClientFormData) => {
    setIsLoading(true)
    
    try {
      if (editingClient) {
        // Update existing client
        const response = await axios.put(`/clients/${editingClient.id}`, {
          name: data.name,
          description: data.description,
        })
        setClients(prev => prev.map(c => c.id === editingClient.id ? response.data : c))
      } else {
        // Create new client and install claude-code
        const response = await axios.post('/clients', {
          name: data.name,
          description: data.description,
        })
        const newClient = response.data
        setClients(prev => [...prev, newClient])
        
        // Start token setup process after installation completes
        if (newClient.status === "pending") {
          // Wait for installation to complete
          const checkInterval = setInterval(async () => {
            const statusRes = await axios.get(`/clients/${newClient.id}/status`)
            if (statusRes.data.status === "pending") {
              // Ready for token setup
              clearInterval(checkInterval)
              handleTokenSetup(newClient)
            }
          }, 2000)
        }
        
        // Notify parent component that a client was added
        if (onClientAdded) {
          onClientAdded()
        }
      }

      form.reset()
      setIsDialogOpen(false)
      setEditingClient(null)
    } catch (error: any) {
      // Failed to save client
      // You could add error handling/toast here
    } finally {
      setIsLoading(false)
    }
  }

  const handleTokenSetup = async (client: Client) => {
    setSetupClient(client)
    setTokenSetupStep("streaming")
    setIsSettingUpToken(true)
    setStreamOutput([])
    
    try {
      // Connect to the streaming setup endpoint
      const eventSource = new EventSource(`/api/claude-sse?client_id=${client.id}`)
      
      eventSource.onmessage = (event) => {
        const data = JSON.parse(event.data)
        
        if (event.type === 'progress' || !event.type) {
          setStreamOutput(prev => [...prev, data.message])
        } else if (event.type === 'complete') {
          setStreamOutput(prev => [...prev, data.message])
          setTokenSetupStep("completed")
          eventSource.close()
          
          // Update client status
          setClients(prevClients => prevClients.map(c => 
            c.id === client.id ? { ...c, status: "active" } : c
          ))
          
          // Auto-close after a delay
          setTimeout(() => {
            setIsSettingUpToken(false)
            setSetupClient(null)
            setStreamOutput([])
            setTokenSetupStep("idle")
          }, 3000)
        } else if (event.type === 'error') {
          setStreamOutput(prev => [...prev, `ERROR: ${data.message}`])
          setTokenSetupStep("error")
          eventSource.close()
        }
      }
      
      eventSource.onerror = (error) => {
        setTokenSetupStep("error")
        setStreamOutput(prev => [...prev, "Connection error occurred"])
        eventSource.close()
      }
      
    } catch (error) {
      setTokenSetupStep("error")
      setIsSettingUpToken(false)
    }
  }

  const closeTokenSetup = () => {
    setIsSettingUpToken(false)
    setSetupClient(null)
    setStreamOutput([])
    setTokenSetupStep("idle")
    
    // Notify parent if completed successfully
    if (tokenSetupStep === "completed" && onClientAdded) {
      onClientAdded()
    }
  }

  const handleEdit = (client: Client) => {
    setEditingClient(client)
    form.reset({
      name: client.name,
      description: client.description || "",
    })
    setIsDialogOpen(true)
  }

  const handleDelete = async (id: string) => {
    try {
      await axios.delete(`/clients/${id}`)
      setClients(prev => prev.filter(c => c.id !== id))
    } catch (error) {
      // Failed to delete client
    }
  }

  const getStatusBadgeColor = (status: Client["status"]) => {
    switch (status) {
      case "active": return "bg-green-100 text-green-800"
      case "pending": return "bg-yellow-100 text-yellow-800"
      case "installing": return "bg-blue-100 text-blue-800"
      case "error": return "bg-red-100 text-red-800"
      default: return "bg-gray-100 text-gray-800"
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Client Setup</h2>
          <p className="text-muted-foreground">
            Configure and manage your Claude Code instances
          </p>
        </div>
        <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
          <DialogTrigger asChild>
            <Button onClick={() => {
              setEditingClient(null)
              form.reset()
            }}>
              <Plus className="mr-2 h-4 w-4" />
              Add Client
            </Button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-[600px]">
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                <DialogHeader>
                  <DialogTitle>
                    {editingClient ? "Edit Client" : "Add New Claude Code Instance"}
                  </DialogTitle>
                  <DialogDescription>
                    Each client will have its own isolated Claude Code instance installed.
                  </DialogDescription>
                </DialogHeader>

                <FormField
                  control={form.control}
                  name="name"
                  rules={{ required: "Name is required" }}
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Client Name *</FormLabel>
                      <FormControl>
                        <Input placeholder="Development Environment" {...field} />
                      </FormControl>
                      <FormDescription>
                        A unique name to identify this Claude Code instance
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="description"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Description</FormLabel>
                      <FormControl>
                        <Textarea 
                          placeholder="Optional description for this client..." 
                          className="resize-none"
                          {...field} 
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <DialogFooter>
                  <Button type="button" variant="outline" onClick={() => {
                    setIsDialogOpen(false)
                    setEditingClient(null)
                    form.reset()
                  }}>
                    Cancel
                  </Button>
                  <Button type="submit" disabled={isLoading}>
                    {editingClient ? "Update Client" : "Add Client"}
                  </Button>
                </DialogFooter>
              </form>
            </Form>
          </DialogContent>
        </Dialog>
      </div>

      {/* Token Setup Dialog */}
      <Dialog open={isSettingUpToken} onOpenChange={setIsSettingUpToken}>
        <DialogContent className="sm:max-w-[600px]">
          <DialogHeader>
            <DialogTitle>Claude Code Authentication Setup</DialogTitle>
            <DialogDescription>
              Complete the authentication to activate your Claude Code instance
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            {tokenSetupStep === "streaming" && (
              <div className="space-y-4">
                <div className="flex items-center space-x-2">
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                  <p className="text-sm text-muted-foreground">Claude CLI is running...</p>
                </div>
                
                <div className="bg-black text-green-400 p-4 rounded-md font-mono text-sm max-h-96 overflow-y-auto">
                  {streamOutput.map((line, index) => (
                    <div key={index} className="whitespace-pre-wrap">
                      {line}
                    </div>
                  ))}
                  <div className="animate-pulse">█</div>
                </div>
              </div>
            )}

            {tokenSetupStep === "completed" && (
              <div className="flex flex-col items-center justify-center py-8 space-y-2">
                <Check className="h-8 w-8 text-green-500" />
                <p className="text-sm text-muted-foreground">Authentication completed successfully!</p>
                <div className="bg-black text-green-400 p-4 rounded-md font-mono text-sm max-h-64 overflow-y-auto w-full">
                  {streamOutput.slice(-10).map((line, index) => (
                    <div key={index} className="whitespace-pre-wrap">
                      {line}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {tokenSetupStep === "error" && (
              <div className="space-y-4">
                <Alert>
                  <AlertDescription>
                    An error occurred during setup. Please check the output below and try again.
                  </AlertDescription>
                </Alert>
                
                <div className="bg-black text-red-400 p-4 rounded-md font-mono text-sm max-h-64 overflow-y-auto">
                  {streamOutput.map((line, index) => (
                    <div key={index} className="whitespace-pre-wrap">
                      {line}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={closeTokenSetup}
              disabled={tokenSetupStep === "streaming"}
            >
              {tokenSetupStep === "completed" ? "Close" : "Cancel"}
            </Button>
            
            {tokenSetupStep === "error" && (
              <Button
                onClick={() => handleTokenSetup(setupClient!)}
                disabled={!setupClient}
              >
                Retry
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {clients.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Settings className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">No clients configured</h3>
            <p className="text-muted-foreground text-center max-w-sm mb-4">
              Get started by adding your first Claude Code instance. Each client runs in an isolated environment.
            </p>
            <Button onClick={() => setIsDialogOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Add Your First Client
            </Button>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Configured Clients</CardTitle>
            <CardDescription>
              Manage your Claude Code instances and their configurations
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Install Path</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-[100px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {clients.map((client) => (
                  <TableRow key={client.id}>
                    <TableCell className="font-medium">{client.name}</TableCell>
                    <TableCell>
                      <Badge className={getStatusBadgeColor(client.status)}>
                        {client.status === "installing" && (
                          <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                        )}
                        {client.status}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-xs font-mono">
                      {client.installPath || `.clients/${client.id}`}
                    </TableCell>
                    <TableCell>{new Date(client.createdAt).toLocaleDateString()}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        {client.status === "installing" && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleTokenSetup(client)}
                          >
                            <Check className="h-4 w-4" />
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleEdit(client)}
                          disabled={client.status === "installing"}
                        >
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleDelete(client.id)}
                          disabled={client.status === "installing"}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

// Add missing Label import
