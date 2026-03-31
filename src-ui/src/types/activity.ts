export type ActivityCategory = "action" | "danger" | "warning" | "error" | "system";

export interface ActivityEntry {
  id: string;
  timestamp: string;
  category: ActivityCategory;
  message: string;
  details?: string;
}
