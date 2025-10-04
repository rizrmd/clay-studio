import { useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Switch } from '@/components/ui/switch';
import { Loader2, Plus, Trash2, Edit2, Clock } from 'lucide-react';
import { analysisApi } from '@/lib/api/analysis';
import { AnalysisSchedule } from '@/lib/store/analysis-store';

interface AnalysisSchedulesDialogProps {
  analysisId: string;
  open: boolean;
  onClose: () => void;
}

export function AnalysisSchedulesDialog({
  analysisId,
  open,
  onClose,
}: AnalysisSchedulesDialogProps) {
  const [schedules, setSchedules] = useState<AnalysisSchedule[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);

  // Form state for create/edit
  const [form, setForm] = useState({
    name: '',
    cron_expression: '',
    is_active: true,
  });

  useEffect(() => {
    if (open) {
      loadSchedules();
    }
  }, [open, analysisId]);

  const loadSchedules = async () => {
    setIsLoading(true);
    setError(null);

    try {
      const data = await analysisApi.getSchedules(analysisId);
      setSchedules(data);
    } catch (err: any) {
      setError(err.message || 'Failed to load schedules');
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreate = async () => {
    if (!form.cron_expression.trim()) {
      setError('Cron expression is required');
      return;
    }

    try {
      await analysisApi.createSchedule({
        analysis_id: analysisId,
        name: form.name || 'Unnamed Schedule',
        cron_expression: form.cron_expression,
        is_active: form.is_active,
      });

      // Reset form and reload
      setForm({ name: '', cron_expression: '', is_active: true });
      setIsCreating(false);
      await loadSchedules();
    } catch (err: any) {
      setError(err.message || 'Failed to create schedule');
    }
  };

  const handleUpdate = async (scheduleId: string) => {
    if (!form.cron_expression.trim()) {
      setError('Cron expression is required');
      return;
    }

    try {
      await analysisApi.updateSchedule(scheduleId, {
        name: form.name || undefined,
        cron_expression: form.cron_expression,
        is_active: form.is_active,
      });

      // Reset form and reload
      setForm({ name: '', cron_expression: '', is_active: true });
      setEditingId(null);
      await loadSchedules();
    } catch (err: any) {
      setError(err.message || 'Failed to update schedule');
    }
  };

  const handleDelete = async (scheduleId: string) => {
    if (!confirm('Are you sure you want to delete this schedule?')) return;

    try {
      await analysisApi.deleteSchedule(scheduleId);
      await loadSchedules();
    } catch (err: any) {
      setError(err.message || 'Failed to delete schedule');
    }
  };

  const handleToggle = async (scheduleId: string, isActive: boolean) => {
    try {
      await analysisApi.toggleSchedule(scheduleId, isActive);
      await loadSchedules();
    } catch (err: any) {
      setError(err.message || 'Failed to toggle schedule');
    }
  };

  const startEdit = (schedule: AnalysisSchedule) => {
    setEditingId(schedule.id);
    setForm({
      name: schedule.name,
      cron_expression: schedule.cron_expression,
      is_active: schedule.is_active,
    });
    setIsCreating(false);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setIsCreating(false);
    setForm({ name: '', cron_expression: '', is_active: true });
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Analysis Schedules</DialogTitle>
          <DialogDescription>
            Manage automated execution schedules for this analysis
          </DialogDescription>
        </DialogHeader>

        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-8 w-8 animate-spin" />
          </div>
        ) : (
          <div className="space-y-4">
            {/* Existing Schedules */}
            {schedules.length > 0 && (
              <div className="space-y-2">
                <h4 className="font-medium text-sm">Existing Schedules</h4>
                {schedules.map((schedule) => (
                  <div
                    key={schedule.id}
                    className="border rounded-lg p-4 space-y-3"
                  >
                    {editingId === schedule.id ? (
                      // Edit Form
                      <div className="space-y-3">
                        <div className="space-y-2">
                          <Label htmlFor="edit-name">Name</Label>
                          <Input
                            id="edit-name"
                            value={form.name}
                            onChange={(e) =>
                              setForm((prev) => ({ ...prev, name: e.target.value }))
                            }
                            placeholder="Schedule name"
                          />
                        </div>
                        <div className="space-y-2">
                          <Label htmlFor="edit-cron">Cron Expression *</Label>
                          <Input
                            id="edit-cron"
                            value={form.cron_expression}
                            onChange={(e) =>
                              setForm((prev) => ({
                                ...prev,
                                cron_expression: e.target.value,
                              }))
                            }
                            placeholder="0 9 * * *"
                          />
                          <p className="text-xs text-muted-foreground">
                            Example: "0 9 * * *" = Daily at 9:00 AM
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={form.is_active}
                            onCheckedChange={(checked) =>
                              setForm((prev) => ({ ...prev, is_active: checked }))
                            }
                          />
                          <Label>Enabled</Label>
                        </div>
                        <div className="flex gap-2">
                          <Button
                            size="sm"
                            onClick={() => handleUpdate(schedule.id)}
                          >
                            Save
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={cancelEdit}
                          >
                            Cancel
                          </Button>
                        </div>
                      </div>
                    ) : (
                      // Display Mode
                      <>
                        <div className="flex items-start justify-between">
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <h5 className="font-medium">{schedule.name}</h5>
                              <Switch
                                checked={schedule.is_active}
                                onCheckedChange={(checked) =>
                                  handleToggle(schedule.id, checked)
                                }
                              />
                            </div>
                            <div className="flex items-center gap-2 mt-1 text-sm text-muted-foreground">
                              <Clock className="h-3 w-3" />
                              <code className="bg-muted px-2 py-0.5 rounded">
                                {schedule.cron_expression}
                              </code>
                            </div>
                            {schedule.next_run && (
                              <p className="text-xs text-muted-foreground mt-1">
                                Next run: {new Date(schedule.next_run).toLocaleString()}
                              </p>
                            )}
                          </div>
                          <div className="flex gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => startEdit(schedule)}
                            >
                              <Edit2 className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleDelete(schedule.id)}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </div>
                      </>
                    )}
                  </div>
                ))}
              </div>
            )}

            {/* Create New Schedule */}
            {!isCreating && !editingId && (
              <Button
                variant="outline"
                onClick={() => setIsCreating(true)}
                className="w-full"
              >
                <Plus className="h-4 w-4 mr-2" />
                Create New Schedule
              </Button>
            )}

            {isCreating && (
              <div className="border rounded-lg p-4 space-y-3">
                <h4 className="font-medium">New Schedule</h4>
                <div className="space-y-2">
                  <Label htmlFor="new-name">Name</Label>
                  <Input
                    id="new-name"
                    value={form.name}
                    onChange={(e) =>
                      setForm((prev) => ({ ...prev, name: e.target.value }))
                    }
                    placeholder="Schedule name"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="new-cron">Cron Expression *</Label>
                  <Input
                    id="new-cron"
                    value={form.cron_expression}
                    onChange={(e) =>
                      setForm((prev) => ({
                        ...prev,
                        cron_expression: e.target.value,
                      }))
                    }
                    placeholder="0 9 * * *"
                  />
                  <p className="text-xs text-muted-foreground">
                    Example: "0 9 * * *" = Daily at 9:00 AM UTC
                  </p>
                  <p className="text-xs text-muted-foreground">
                    Format: minute hour day month weekday
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={form.is_active}
                    onCheckedChange={(checked) =>
                      setForm((prev) => ({ ...prev, is_active: checked }))
                    }
                  />
                  <Label>Enabled</Label>
                </div>
                <div className="flex gap-2">
                  <Button onClick={handleCreate}>Create</Button>
                  <Button variant="outline" onClick={cancelEdit}>
                    Cancel
                  </Button>
                </div>
              </div>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}